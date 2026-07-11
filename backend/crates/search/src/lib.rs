//! Federated public search across catalogue, community content, and discovery objects.
//!
//! Search engines provide ranked candidate IDs only. Each owning domain must
//! reconstruct hits from PostgreSQL and enforce current visibility before this
//! crate combines the typed results.

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shared::{AppError, AppResult, AppState};

mod quality;

pub use quality::{SearchHighlightDto, SearchHighlightRangeDto};

const DEFAULT_LIMIT: usize = 10;
const MAX_LIMIT: usize = 30;
const MAX_VISIBLE_RESULTS: usize = 240;

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

    const fn cursor_key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Course => "course",
            Self::Teacher => "teacher",
            Self::Review => "review",
            Self::Thread => "thread",
            Self::User => "user",
            Self::Board => "board",
            Self::Tag => "tag",
        }
    }

    const fn result_scope(self) -> Option<&'static str> {
        match self {
            Self::All => None,
            Self::Course | Self::Teacher => Some("course"),
            Self::Review => Some("review"),
            Self::Thread => Some("thread"),
            Self::User => Some("user"),
            Self::Board => Some("board"),
            Self::Tag => Some("tag"),
        }
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
    cursor: Option<String>,
}

fn default_limit() -> usize {
    DEFAULT_LIMIT
}

struct ValidatedSearch<'a> {
    query: &'a str,
    scope: SearchScope,
    limit: usize,
    offset: usize,
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
    let scope = SearchScope::parse(params.query_type.as_deref())?;
    let offset = match params.cursor.as_deref() {
        Some(_) if scope == SearchScope::All => {
            return Err("cursor requires a specific search type".into())
        }
        Some(cursor) => decode_cursor(cursor, query, scope)?,
        None => 0,
    };
    let limit = params.limit.min(MAX_VISIBLE_RESULTS.saturating_sub(offset));
    if limit == 0 {
        return Err("cursor is outside the searchable result window".into());
    }
    Ok(ValidatedSearch { query, scope, limit, offset })
}

fn cursor_fingerprint(query: &str, scope: SearchScope) -> String {
    let mut hasher = Sha256::new();
    hasher.update(scope.cursor_key().as_bytes());
    hasher.update([0]);
    hasher.update(query.as_bytes());
    let digest = hasher.finalize();
    hex::encode(&digest[..12])
}

fn encode_cursor(query: &str, scope: SearchScope, offset: usize) -> String {
    URL_SAFE_NO_PAD.encode(format!(
        "1|{}|{offset}|{}",
        scope.cursor_key(),
        cursor_fingerprint(query, scope)
    ))
}

fn decode_cursor(cursor: &str, query: &str, scope: SearchScope) -> Result<usize, String> {
    if cursor.len() > 256 {
        return Err("invalid search cursor".into());
    }
    let decoded = URL_SAFE_NO_PAD.decode(cursor).map_err(|_| "invalid search cursor")?;
    let decoded = std::str::from_utf8(&decoded).map_err(|_| "invalid search cursor")?;
    let mut parts = decoded.split('|');
    let version = parts.next();
    let cursor_scope = parts.next();
    let offset = parts.next().and_then(|value| value.parse::<usize>().ok());
    let fingerprint = parts.next();
    if version != Some("1")
        || cursor_scope != Some(scope.cursor_key())
        || fingerprint != Some(cursor_fingerprint(query, scope).as_str())
        || parts.next().is_some()
    {
        return Err("invalid search cursor".into());
    }
    let offset = offset.ok_or_else(|| "invalid search cursor".to_owned())?;
    if offset >= MAX_VISIBLE_RESULTS {
        return Err("search cursor exceeds the result window".into());
    }
    Ok(offset)
}

impl ValidatedSearch<'_> {
    fn fetch_limit(&self) -> usize {
        self.offset + self.limit + usize::from(self.offset + self.limit < MAX_VISIBLE_RESULTS)
    }
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
    pub next_cursor: Option<String>,
    pub has_more: bool,
    pub has_more_scopes: Vec<String>,
    pub failed_scopes: Vec<String>,
    pub highlights: Vec<SearchHighlightDto>,
    pub suggested_query: Option<String>,
}

struct ResultPage<T> {
    items: Vec<T>,
    has_more: bool,
}

fn page_results<T>(items: Vec<T>, search: &ValidatedSearch<'_>) -> ResultPage<T> {
    let mut items =
        items.into_iter().skip(search.offset).take(search.limit + 1).collect::<Vec<_>>();
    let has_more = items.len() > search.limit
        && search.offset.saturating_add(search.limit) < MAX_VISIBLE_RESULTS;
    if has_more {
        items.truncate(search.limit);
    }
    ResultPage { items, has_more }
}

fn recover_section<T>(
    result: AppResult<Vec<T>>,
    is_included: bool,
    scope: &'static str,
    failed_scopes: &mut Vec<String>,
) -> Vec<T> {
    match result {
        Ok(items) => items,
        Err(error) if is_included => {
            tracing::warn!(?error, search_scope = scope, "federated search section failed");
            failed_scopes.push(scope.to_owned());
            Vec::new()
        }
        Err(error) => {
            tracing::warn!(?error, search_scope = scope, "excluded search section failed");
            Vec::new()
        }
    }
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
        search.fetch_limit(),
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
        search.fetch_limit(),
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
        search.fetch_limit(),
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
        search.fetch_limit(),
    )
    .await?;
    forum::discovery::load_user_hits(&state.db, &candidates, viewer_id, search.fetch_limit()).await
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
        search.fetch_limit(),
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
        search.fetch_limit(),
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
    let mut failed_scopes = Vec::new();
    let courses = page_results(
        recover_section(courses, search.scope.includes_courses(), "course", &mut failed_scopes),
        &search,
    );
    let reviews = page_results(
        recover_section(reviews, search.scope.includes_reviews(), "review", &mut failed_scopes),
        &search,
    );
    let threads = page_results(
        recover_section(threads, search.scope.includes_threads(), "thread", &mut failed_scopes),
        &search,
    );
    let users = page_results(
        recover_section(users, search.scope.includes_users(), "user", &mut failed_scopes),
        &search,
    );
    let boards = page_results(
        recover_section(boards, search.scope.includes_boards(), "board", &mut failed_scopes),
        &search,
    );
    let tags = page_results(
        recover_section(tags, search.scope.includes_tags(), "tag", &mut failed_scopes),
        &search,
    );
    let scoped_page = match search.scope.result_scope() {
        Some("course") => Some((courses.has_more, courses.items.len())),
        Some("review") => Some((reviews.has_more, reviews.items.len())),
        Some("thread") => Some((threads.has_more, threads.items.len())),
        Some("user") => Some((users.has_more, users.items.len())),
        Some("board") => Some((boards.has_more, boards.items.len())),
        Some("tag") => Some((tags.has_more, tags.items.len())),
        _ => None,
    };
    let mut has_more_scopes = Vec::new();
    for (scope, has_more) in [
        ("course", courses.has_more),
        ("review", reviews.has_more),
        ("thread", threads.has_more),
        ("user", users.has_more),
        ("board", boards.has_more),
        ("tag", tags.has_more),
    ] {
        if has_more {
            has_more_scopes.push(scope.to_owned());
        }
    }
    let (has_more, next_cursor) = scoped_page.map_or((false, None), |(has_more, item_count)| {
        let cursor = has_more.then(|| {
            encode_cursor(search.query, search.scope, search.offset.saturating_add(item_count))
        });
        (has_more, cursor)
    });
    let quality = quality::build_search_quality(
        search.query,
        &courses.items,
        &reviews.items,
        &threads.items,
        &users.items,
        &boards.items,
        &tags.items,
    );
    Ok(Json(SearchResultDto {
        courses: courses.items,
        reviews: reviews.items,
        threads: threads.items,
        users: users.items,
        boards: boards.items,
        tags: tags.items,
        next_cursor,
        has_more,
        has_more_scopes,
        failed_scopes,
        highlights: quality.highlights,
        suggested_query: quality.suggested_query,
    }))
}

/// Routes owned by the federated search domain.
pub fn routes(state: AppState) -> Router {
    Router::new().route("/api/v2/search", get(global_search)).with_state(state)
}

#[cfg(test)]
mod tests {
    use super::{encode_cursor, page_results, validate_query, SearchQuery, SearchScope};

    #[test]
    fn validates_scope_query_and_limit() {
        let valid = SearchQuery {
            q: "  数据结构  ".into(),
            query_type: Some("thread".into()),
            limit: 12,
            cursor: None,
        };
        let search = validate_query(&valid).expect("valid search");
        assert_eq!(search.query, "数据结构");
        assert_eq!(search.scope, SearchScope::Thread);
        assert_eq!(search.limit, 12);

        for invalid in [
            SearchQuery { q: "x".into(), query_type: None, limit: 10, cursor: None },
            SearchQuery {
                q: "valid".into(),
                query_type: Some("post".into()),
                limit: 10,
                cursor: None,
            },
            SearchQuery { q: "valid".into(), query_type: None, limit: 31, cursor: None },
        ] {
            assert!(validate_query(&invalid).is_err());
        }
        assert!(validate_query(&SearchQuery {
            q: "数据结构".into(),
            query_type: Some("thread".into()),
            limit: 12,
            cursor: Some("x".repeat(257)),
        })
        .is_err());
    }

    #[test]
    fn scoped_cursor_is_bound_to_query_scope_and_bounded_window() {
        let cursor = encode_cursor("数据结构", SearchScope::Thread, 12);
        let params = SearchQuery {
            q: "数据结构".into(),
            query_type: Some("thread".into()),
            limit: 12,
            cursor: Some(cursor.clone()),
        };
        let search = validate_query(&params).expect("matching cursor");
        assert_eq!(search.offset, 12);
        assert_eq!(search.fetch_limit(), 25);

        for invalid in [
            SearchQuery {
                q: "另一查询".into(),
                query_type: Some("thread".into()),
                limit: 12,
                cursor: Some(cursor.clone()),
            },
            SearchQuery {
                q: "数据结构".into(),
                query_type: Some("review".into()),
                limit: 12,
                cursor: Some(cursor.clone()),
            },
            SearchQuery {
                q: "数据结构".into(),
                query_type: Some("all".into()),
                limit: 12,
                cursor: Some(cursor.clone()),
            },
        ] {
            assert!(validate_query(&invalid).is_err());
        }
    }

    #[test]
    fn result_pages_use_lookahead_without_exceeding_the_search_window() {
        let params = SearchQuery {
            q: "数据结构".into(),
            query_type: Some("thread".into()),
            limit: 3,
            cursor: Some(encode_cursor("数据结构", SearchScope::Thread, 2)),
        };
        let search = validate_query(&params).expect("valid page");
        let page = page_results(vec![0, 1, 2, 3, 4, 5], &search);
        assert_eq!(page.items, vec![2, 3, 4]);
        assert!(page.has_more);
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
