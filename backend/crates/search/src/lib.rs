//! Federated public search across catalogue, community content, and discovery objects.
//!
//! Search engines provide ranked candidate IDs only. Each owning domain must
//! reconstruct hits from PostgreSQL and enforce current visibility before this
//! crate combines the typed results.

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shared::{AppError, AppResult, AppState};

const DEFAULT_LIMIT: usize = 10;
const MAX_LIMIT: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchScope {
    All,
    Course,
    Teacher,
    Review,
    Thread,
    User,
    Board,
    Tag,
}

impl SearchScope {
    fn parse(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("all") {
            "all" => Ok(Self::All),
            "course" => Ok(Self::Course),
            "teacher" => Ok(Self::Teacher),
            "review" => Ok(Self::Review),
            "thread" => Ok(Self::Thread),
            "user" => Ok(Self::User),
            "board" => Ok(Self::Board),
            "tag" => Ok(Self::Tag),
            _ => {
                Err("type must be course, teacher, review, thread, user, board, tag, or all".into())
            }
        }
    }

    fn includes_courses(self) -> bool {
        matches!(self, Self::All | Self::Course | Self::Teacher)
    }

    fn includes_reviews(self) -> bool {
        matches!(self, Self::All | Self::Review)
    }

    fn includes_threads(self) -> bool {
        matches!(self, Self::All | Self::Thread)
    }

    fn includes_users(self) -> bool {
        matches!(self, Self::All | Self::User)
    }

    fn includes_boards(self) -> bool {
        matches!(self, Self::All | Self::Board)
    }

    fn includes_tags(self) -> bool {
        matches!(self, Self::All | Self::Tag)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchQuery {
    q: String,
    #[serde(rename = "type")]
    query_type: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    DEFAULT_LIMIT
}

struct ValidatedSearch<'a> {
    query: &'a str,
    scope: SearchScope,
    limit: usize,
}

fn validate_query(params: &SearchQuery) -> Result<ValidatedSearch<'_>, String> {
    let query = params.q.trim();
    let query_length = query.chars().count();
    if !(2..=100).contains(&query_length) || query.chars().any(char::is_control) {
        return Err("q must contain between 2 and 100 non-control characters".into());
    }
    if !(1..=MAX_LIMIT).contains(&params.limit) {
        return Err(format!("limit must be between 1 and {MAX_LIMIT}"));
    }
    Ok(ValidatedSearch {
        query,
        scope: SearchScope::parse(params.query_type.as_deref())?,
        limit: params.limit,
    })
}

/// Typed federated search response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultDto {
    pub courses: Vec<courses::public_search::CourseSearchHit>,
    pub reviews: Vec<reviews::search::ReviewSearchHit>,
    pub threads: Vec<forum::meili::ForumThreadDoc>,
    pub users: Vec<forum::discovery::UserSearchHit>,
    pub boards: Vec<forum::discovery::BoardSearchHit>,
    pub tags: Vec<forum::discovery::TagSearchHit>,
}

fn rate_limit_key(headers: &HeaderMap) -> String {
    let identifier = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    hex::encode(Sha256::digest(identifier.as_bytes()))
}

async fn search_courses_if(
    state: &AppState,
    search: &ValidatedSearch<'_>,
) -> AppResult<Vec<courses::public_search::CourseSearchHit>> {
    if !search.scope.includes_courses() {
        return Ok(Vec::new());
    }
    courses::public_search::search_courses(
        &state.db,
        &state.meili_url,
        &state.meili_master_key,
        search.query,
        search.limit,
    )
    .await
}

async fn search_reviews_if(
    state: &AppState,
    search: &ValidatedSearch<'_>,
) -> AppResult<Vec<reviews::search::ReviewSearchHit>> {
    if !search.scope.includes_reviews() {
        return Ok(Vec::new());
    }
    reviews::search::search_reviews(
        &state.db,
        &state.meili_url,
        &state.meili_master_key,
        search.query,
        search.limit,
    )
    .await
}

async fn search_threads_if(
    state: &AppState,
    search: &ValidatedSearch<'_>,
    viewer_id: Option<i64>,
) -> AppResult<Vec<forum::meili::ForumThreadDoc>> {
    if !search.scope.includes_threads() {
        return Ok(Vec::new());
    }
    forum::meili::search_threads(
        &state.db,
        &state.meili_url,
        &state.meili_master_key,
        search.query,
        search.limit,
        viewer_id,
    )
    .await
}

async fn search_users_if(
    state: &AppState,
    search: &ValidatedSearch<'_>,
    viewer_id: Option<i64>,
) -> AppResult<Vec<forum::discovery::UserSearchHit>> {
    if !search.scope.includes_users() {
        return Ok(Vec::new());
    }
    let candidates = identity::public_search::search_user_ids(
        &state.meili_url,
        &state.meili_master_key,
        search.query,
        search.limit,
    )
    .await?;
    forum::discovery::load_user_hits(&state.db, &candidates, viewer_id, search.limit).await
}

async fn search_boards_if(
    state: &AppState,
    search: &ValidatedSearch<'_>,
) -> AppResult<Vec<forum::discovery::BoardSearchHit>> {
    if !search.scope.includes_boards() {
        return Ok(Vec::new());
    }
    forum::discovery::search_boards(
        &state.db,
        &state.meili_url,
        &state.meili_master_key,
        search.query,
        search.limit,
    )
    .await
}

async fn search_tags_if(
    state: &AppState,
    search: &ValidatedSearch<'_>,
) -> AppResult<Vec<forum::discovery::TagSearchHit>> {
    if !search.scope.includes_tags() {
        return Ok(Vec::new());
    }
    forum::discovery::search_tags(
        &state.db,
        &state.meili_url,
        &state.meili_master_key,
        search.query,
        search.limit,
    )
    .await
}

async fn global_search(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<SearchQuery>,
) -> AppResult<Json<SearchResultDto>> {
    let search = validate_query(&params).map_err(AppError::BadRequest)?;
    let viewer = identity::auth_middleware::authenticate_optional(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    let viewer_id = viewer.as_ref().map(|account| account.id);
    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        "search",
        &rate_limit_key(&headers),
        30,
        10,
    )
    .await?;

    let (courses, reviews, threads, users, boards, tags) = tokio::join!(
        search_courses_if(&state, &search),
        search_reviews_if(&state, &search),
        search_threads_if(&state, &search, viewer_id),
        search_users_if(&state, &search, viewer_id),
        search_boards_if(&state, &search),
        search_tags_if(&state, &search),
    );
    Ok(Json(SearchResultDto {
        courses: courses?,
        reviews: reviews?,
        threads: threads?,
        users: users?,
        boards: boards?,
        tags: tags?,
    }))
}

/// Routes owned by the federated search domain.
pub fn routes(state: AppState) -> Router {
    Router::new().route("/api/v2/search", get(global_search)).with_state(state)
}

#[cfg(test)]
mod tests {
    use super::{validate_query, SearchQuery, SearchScope};

    #[test]
    fn validates_scope_query_and_limit() {
        let valid =
            SearchQuery {
                q: "  数据结构  ".into(), query_type: Some("thread".into()), limit: 12
            };
        let search = validate_query(&valid).expect("valid search");
        assert_eq!(search.query, "数据结构");
        assert_eq!(search.scope, SearchScope::Thread);
        assert_eq!(search.limit, 12);

        for invalid in [
            SearchQuery { q: "x".into(), query_type: None, limit: 10 },
            SearchQuery { q: "valid".into(), query_type: Some("post".into()), limit: 10 },
            SearchQuery { q: "valid".into(), query_type: None, limit: 31 },
        ] {
            assert!(validate_query(&invalid).is_err());
        }
    }

    #[test]
    fn scopes_select_only_the_requested_domains() {
        assert!(SearchScope::All.includes_courses());
        assert!(SearchScope::All.includes_reviews());
        assert!(SearchScope::All.includes_threads());
        assert!(SearchScope::All.includes_users());
        assert!(SearchScope::All.includes_boards());
        assert!(SearchScope::All.includes_tags());

        assert!(SearchScope::Teacher.includes_courses());
        assert!(!SearchScope::Teacher.includes_reviews());
        assert!(!SearchScope::Teacher.includes_threads());

        assert!(SearchScope::Review.includes_reviews());
        assert!(!SearchScope::Review.includes_courses());
        assert!(!SearchScope::Review.includes_threads());

        assert!(SearchScope::User.includes_users());
        assert!(!SearchScope::User.includes_threads());
        assert!(SearchScope::Board.includes_boards());
        assert!(SearchScope::Tag.includes_tags());

        assert!(SearchScope::Thread.includes_threads());
        assert!(!SearchScope::Thread.includes_courses());
        assert!(!SearchScope::Thread.includes_reviews());
    }
}
