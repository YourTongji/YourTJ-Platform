//! HTTP handlers for the selection (选课) domain.
//!
//! Canonical APIs use teachingClassId-backed offering identity and bounded Page
//! envelopes. Legacy routes remain bounded teaching-class-id adapters.

use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shared::{AppError, AppResult, AppState, Page};

use crate::error::CoursesError;
use crate::selection::dto::{
    CalendarDto, CampusDto, CourseNatureDto, FacultyDto, LatestUpdateDto, MajorDto,
    SelectionCourseDto, TimeSlotDto,
};
use crate::selection::models::{SelectionCourseRow, TimeslotRow};
use crate::selection_repo::{self, OfferingCursor, OfferingFilter};

const SELECTION_STALE_AFTER_HOURS: i64 = 168;
const MAX_SEARCH_WINDOW: usize = 1_000;
const SEARCH_CANDIDATE_BATCH: usize = 200;
const MAX_SEARCH_BATCHES_PER_PAGE: usize = 5;

fn default_limit() -> i64 {
    20
}

fn default_true() -> bool {
    true
}

fn next_search_batch_size(search_offset: usize, completed_batches: usize) -> Option<usize> {
    if search_offset >= MAX_SEARCH_WINDOW || completed_batches >= MAX_SEARCH_BATCHES_PER_PAGE {
        return None;
    }
    Some(SEARCH_CANDIDATE_BATCH.min(MAX_SEARCH_WINDOW - search_offset))
}

fn bad_request(message: impl Into<String>) -> AppError {
    AppError::BadRequest(message.into())
}

async fn versioned_cache_id(state: &AppState, prefix: &str, id: &str) -> String {
    let version = shared::cache::current_version_opt(state.redis.as_ref(), prefix, "all").await;
    format!("g{version}:{id}")
}

fn row_to_course_dto(row: SelectionCourseRow) -> SelectionCourseDto {
    let offering_id = row.id.to_string();
    SelectionCourseDto {
        id: offering_id.clone(),
        offering_id,
        code: row.code,
        teaching_class_code: row.teaching_class_code,
        name: row.name,
        credit: row.credit,
        nature_id: row.nature_id.map(|value| value.to_string()),
        calendar_id: row.calendar_id.to_string(),
        campus_id: row.campus_id.map(|value| value.to_string()),
        faculty_name: row.faculty_name,
        teaching_language: row.teaching_language,
        teacher_name: row.teacher_name,
        teacher_names: row.teacher_names.unwrap_or_default(),
        start_week: row.start_week,
        end_week: row.end_week,
        weeks_unknown: row.weeks_unknown,
        schedule_unknown: row.schedule_unknown,
        status: row.status,
        catalogue_course_id: row.catalogue_course_id.map(|value| value.to_string()),
        review_count: row.review_count,
        review_avg: row.review_avg,
        review_scope: row.review_scope,
    }
}

fn row_to_timeslot_dto(row: TimeslotRow) -> TimeSlotDto {
    let offering_id = row.course_id.to_string();
    TimeSlotDto {
        offering_id: offering_id.clone(),
        course_id: offering_id,
        teacher_name: row.teacher_name,
        weekday: row.weekday,
        start_slot: row.start_slot,
        end_slot: row.end_slot,
        weeks: row.weeks,
        week_numbers: row.week_numbers,
        weeks_unknown: row.weeks_unknown,
        location: row.location,
        location_unknown: row.location_unknown,
    }
}

pub async fn selection_calendars(state: State<AppState>) -> AppResult<Json<Vec<CalendarDto>>> {
    let key = versioned_cache_id(&state, "selection-calendars", "all").await;
    let items =
        shared::cache::cached_json(state.redis.as_ref(), "selection-calendars", &key, 600, async {
            Ok::<_, CoursesError>(
                selection_repo::list_calendars(&state.db)
                    .await?
                    .into_iter()
                    .map(|row| CalendarDto {
                        id: row.id.to_string(),
                        name: row.name,
                        is_current: row.is_current,
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .await?;
    Ok(Json(items))
}

pub async fn selection_campuses(state: State<AppState>) -> AppResult<Json<Vec<CampusDto>>> {
    let key = versioned_cache_id(&state, "selection-campuses", "all").await;
    let items =
        shared::cache::cached_json(state.redis.as_ref(), "selection-campuses", &key, 600, async {
            Ok::<_, CoursesError>(
                selection_repo::list_campuses(&state.db)
                    .await?
                    .into_iter()
                    .map(|row| CampusDto { id: row.id.to_string(), name: row.name })
                    .collect::<Vec<_>>(),
            )
        })
        .await?;
    Ok(Json(items))
}

pub async fn selection_faculties(state: State<AppState>) -> AppResult<Json<Vec<FacultyDto>>> {
    let key = versioned_cache_id(&state, "selection-faculties", "all").await;
    let items =
        shared::cache::cached_json(state.redis.as_ref(), "selection-faculties", &key, 600, async {
            Ok::<_, CoursesError>(
                selection_repo::list_faculties(&state.db)
                    .await?
                    .into_iter()
                    .map(|row| FacultyDto {
                        id: row.id.to_string(),
                        name: row.name,
                        campus_id: row.campus_id.map(|value| value.to_string()),
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .await?;
    Ok(Json(items))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradesQuery {
    pub calendar_id: i64,
}

fn validate_calendar_id(calendar_id: i64) -> AppResult<()> {
    if calendar_id <= 0 {
        return Err(AppError::BadRequest("invalid selection calendar id".into()));
    }
    Ok(())
}

fn parse_selection_query<T>(query: Result<Query<T>, QueryRejection>) -> AppResult<T> {
    query
        .map(|Query(params)| params)
        .map_err(|_| AppError::BadRequest("invalid selection query".into()))
}

pub async fn selection_grades(
    state: State<AppState>,
    query: Result<Query<GradesQuery>, QueryRejection>,
) -> AppResult<Json<Vec<String>>> {
    let params = parse_selection_query(query)?;
    validate_calendar_id(params.calendar_id)?;
    let raw_key = params.calendar_id.to_string();
    let key = versioned_cache_id(&state, "selection-grades", &raw_key).await;
    let grades =
        shared::cache::cached_json(state.redis.as_ref(), "selection-grades", &key, 600, async {
            selection_repo::list_grades(&state.db, params.calendar_id).await
        })
        .await?;
    Ok(Json(grades))
}

#[derive(Debug, Deserialize)]
pub struct MajorsQuery {
    #[serde(rename = "calendarId")]
    pub calendar_id: i64,
    pub grade: String,
}

pub async fn selection_majors(
    state: State<AppState>,
    query: Result<Query<MajorsQuery>, QueryRejection>,
) -> AppResult<Json<Vec<MajorDto>>> {
    let params = parse_selection_query(query)?;
    validate_calendar_id(params.calendar_id)?;
    let grade = params.grade.trim();
    if grade.is_empty() || grade.chars().count() > 16 || grade.chars().any(char::is_control) {
        return Err(bad_request("grade must contain 1-16 non-control characters"));
    }
    let key =
        versioned_cache_id(&state, "selection-majors", &format!("{}:{grade}", params.calendar_id))
            .await;
    let items =
        shared::cache::cached_json(state.redis.as_ref(), "selection-majors", &key, 600, async {
            Ok::<_, CoursesError>(
                selection_repo::list_majors(&state.db, params.calendar_id, grade)
                    .await?
                    .into_iter()
                    .map(|row| MajorDto {
                        id: row.id.to_string(),
                        name: row.name,
                        faculty_id: row.faculty_id.map(|value| value.to_string()),
                        grade: row.grade,
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .await?;
    Ok(Json(items))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseNaturesQuery {
    pub calendar_id: i64,
}

pub async fn selection_course_natures(
    state: State<AppState>,
    query: Result<Query<CourseNaturesQuery>, QueryRejection>,
) -> AppResult<Json<Vec<CourseNatureDto>>> {
    let params = parse_selection_query(query)?;
    validate_calendar_id(params.calendar_id)?;
    let key =
        versioned_cache_id(&state, "selection-natures", &params.calendar_id.to_string()).await;
    let items =
        shared::cache::cached_json(state.redis.as_ref(), "selection-natures", &key, 600, async {
            Ok::<_, CoursesError>(
                selection_repo::list_course_natures(&state.db, params.calendar_id)
                    .await?
                    .into_iter()
                    .map(|row| CourseNatureDto { id: row.id.to_string(), name: row.name })
                    .collect::<Vec<_>>(),
            )
        })
        .await?;
    Ok(Json(items))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferingsQuery {
    pub q: Option<String>,
    pub calendar_id: i64,
    pub major_id: Option<i64>,
    pub grade: Option<String>,
    pub nature_id: Option<i64>,
    pub campus_id: Option<i64>,
    pub course_code: Option<String>,
    pub weekday: Option<i32>,
    pub start_slot: Option<i32>,
    pub end_slot: Option<i32>,
    pub week: Option<i32>,
    #[serde(default = "default_true")]
    pub include_unknown_schedule: bool,
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OfferingCursorPayload {
    version: u8,
    mode: String,
    fingerprint: String,
    code: Option<String>,
    id: Option<i64>,
    offset: Option<usize>,
}

fn normalized_query(params: &OfferingsQuery) -> Option<String> {
    params.q.as_deref().map(str::trim).filter(|value| !value.is_empty()).map(str::to_lowercase)
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

fn filter_fingerprint(params: &OfferingsQuery) -> String {
    let value = serde_json::json!({
        "q": normalized_query(params),
        "calendarId": params.calendar_id,
        "majorId": params.major_id,
        "grade": params.grade.as_deref().map(str::trim),
        "natureId": params.nature_id,
        "campusId": params.campus_id,
        "courseCode": params.course_code.as_deref().map(str::trim),
        "weekday": params.weekday,
        "startSlot": params.start_slot,
        "endSlot": params.end_slot,
        "week": params.week,
        "includeUnknownSchedule": params.include_unknown_schedule,
    });
    let mut hasher = Sha256::new();
    hasher.update(value.to_string().as_bytes());
    hex::encode(&hasher.finalize()[..12])
}

fn encode_cursor(payload: &OfferingCursorPayload) -> AppResult<String> {
    let bytes = serde_json::to_vec(payload)
        .map_err(|error| AppError::Internal(anyhow::Error::new(error)))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn decode_cursor(
    encoded: &str,
    expected_mode: &str,
    expected_fingerprint: &str,
) -> AppResult<OfferingCursorPayload> {
    if encoded.len() > 1_024 {
        return Err(bad_request("invalid offering cursor"));
    }
    let bytes =
        URL_SAFE_NO_PAD.decode(encoded).map_err(|_| bad_request("invalid offering cursor"))?;
    let payload: OfferingCursorPayload =
        serde_json::from_slice(&bytes).map_err(|_| bad_request("invalid offering cursor"))?;
    if payload.version != 1
        || payload.mode != expected_mode
        || payload.fingerprint != expected_fingerprint
    {
        return Err(bad_request("offering cursor does not match this query"));
    }
    Ok(payload)
}

fn validate_offerings_query(params: &OfferingsQuery) -> AppResult<OfferingFilter> {
    if !(1..=100).contains(&params.limit) {
        return Err(bad_request("limit must be between 1 and 100"));
    }
    validate_calendar_id(params.calendar_id)?;
    for (name, value) in [
        ("majorId", params.major_id),
        ("natureId", params.nature_id),
        ("campusId", params.campus_id),
    ] {
        if value.is_some_and(|value| value <= 0) {
            return Err(bad_request(format!("{name} must be positive")));
        }
    }
    let normalized_query = normalized_query(params);
    if params.q.is_some() && normalized_query.is_none() {
        return Err(bad_request("q must contain 1-100 non-control characters"));
    }
    if let Some(query) = normalized_query {
        if query.chars().count() > 100 || query.chars().any(char::is_control) {
            return Err(bad_request("q must contain 1-100 non-control characters"));
        }
    }
    let grade = params.grade.as_deref().map(str::trim).filter(|value| !value.is_empty());
    if params.grade.is_some() && grade.is_none()
        || grade
            .is_some_and(|value| value.chars().count() > 16 || value.chars().any(char::is_control))
    {
        return Err(bad_request("grade must contain 1-16 non-control characters"));
    }
    let course_code =
        params.course_code.as_deref().map(str::trim).filter(|value| !value.is_empty());
    if params.course_code.is_some() && course_code.is_none()
        || course_code
            .is_some_and(|value| value.chars().count() > 64 || value.chars().any(char::is_control))
    {
        return Err(bad_request("courseCode must contain 1-64 non-control characters"));
    }
    let time_fields =
        [params.weekday.is_some(), params.start_slot.is_some(), params.end_slot.is_some()];
    if time_fields.iter().any(|present| *present) && !time_fields.iter().all(|present| *present) {
        return Err(bad_request("weekday, startSlot, and endSlot must be supplied together"));
    }
    if params.weekday.is_some_and(|value| !(1..=7).contains(&value)) {
        return Err(bad_request("weekday must be between 1 and 7"));
    }
    if params.start_slot.is_some_and(|value| !(1..=20).contains(&value))
        || params.end_slot.is_some_and(|value| !(1..=20).contains(&value))
        || matches!((params.start_slot, params.end_slot), (Some(start), Some(end)) if end < start)
    {
        return Err(bad_request("slot range must be ordered within 1-20"));
    }
    if params.week.is_some_and(|value| !(1..=30).contains(&value)) {
        return Err(bad_request("week must be between 1 and 30"));
    }
    if params.week.is_some() && params.weekday.is_none() {
        return Err(bad_request("week requires a weekday and slot range"));
    }
    if !params.include_unknown_schedule && params.weekday.is_none() {
        return Err(bad_request("includeUnknownSchedule=false requires a weekday and slot range"));
    }
    Ok(OfferingFilter {
        calendar_id: Some(params.calendar_id),
        major_id: params.major_id,
        grade: grade.map(str::to_owned),
        nature_id: params.nature_id,
        campus_id: params.campus_id,
        course_code: course_code.map(str::to_owned),
        weekday: params.weekday,
        start_slot: params.start_slot,
        end_slot: params.end_slot,
        week: params.week,
        include_unknown_schedule: params.include_unknown_schedule,
    })
}

async fn fetch_offering_page(
    state: &AppState,
    params: &OfferingsQuery,
) -> AppResult<Page<SelectionCourseDto>> {
    let filter = validate_offerings_query(params)?;
    let fingerprint = filter_fingerprint(params);
    if let Some(query) = normalized_query(params) {
        if state.meili_url.trim().is_empty() {
            return Err(AppError::ServiceUnavailable);
        }
        if !crate::meili::projection_is_ready(&state.db, "selection").await? {
            return Err(AppError::ServiceUnavailable);
        }
        let offset = match params.cursor.as_deref() {
            Some(cursor) => decode_cursor(cursor, "search", &fingerprint)?
                .offset
                .ok_or_else(|| bad_request("invalid search cursor"))?,
            None => 0,
        };
        if offset >= MAX_SEARCH_WINDOW {
            return Err(bad_request("search cursor exceeds the result window"));
        }
        let limit = usize::try_from(params.limit)
            .map_err(|error| AppError::Internal(anyhow::Error::new(error)))?;
        let mut search_offset = offset;
        let mut has_more = true;
        let mut rows = Vec::with_capacity(limit);
        let mut batches = 0;
        while rows.len() < limit && has_more {
            let Some(fetch_limit) = next_search_batch_size(search_offset, batches) else {
                break;
            };
            batches += 1;
            let candidates = crate::meili::search_selection_offering_ids(
                &state.meili_url,
                &state.meili_master_key,
                &query,
                &filter,
                search_offset,
                fetch_limit,
            )
            .await?;
            if candidates.consumed == 0 {
                has_more = false;
                break;
            }
            let hydrated = selection_repo::find_offerings_by_candidate_ids(
                &state.db,
                &filter,
                &candidates.ids,
            )
            .await?;
            let remaining = limit - rows.len();
            if hydrated.len() > remaining {
                let selected = hydrated.into_iter().take(remaining).collect::<Vec<_>>();
                let Some(last_selected) = selected.last() else {
                    return Err(AppError::Internal(anyhow::anyhow!(
                        "selection search produced an empty bounded page"
                    )));
                };
                let consumed_through = match candidates
                    .ids
                    .iter()
                    .position(|candidate_id| *candidate_id == last_selected.id)
                {
                    Some(position) => candidates.consumed_through[position],
                    None => {
                        return Err(AppError::Internal(anyhow::anyhow!(
                            "selection search ordering could not be reconciled"
                        )))
                    }
                };
                rows.extend(selected);
                search_offset += consumed_through;
                has_more = consumed_through < candidates.consumed || candidates.has_more;
                break;
            }
            rows.extend(hydrated);
            search_offset += candidates.consumed;
            has_more = candidates.has_more;
        }
        let next_cursor = if has_more && search_offset > offset && search_offset < MAX_SEARCH_WINDOW
        {
            Some(encode_cursor(&OfferingCursorPayload {
                version: 1,
                mode: "search".into(),
                fingerprint: fingerprint.clone(),
                code: None,
                id: None,
                offset: Some(search_offset),
            })?)
        } else {
            None
        };
        return Ok(Page::new(rows.into_iter().map(row_to_course_dto).collect(), next_cursor));
    }

    let cursor = params
        .cursor
        .as_deref()
        .map(|cursor| decode_cursor(cursor, "browse", &fingerprint))
        .transpose()?
        .map(|payload| {
            Ok::<_, AppError>(OfferingCursor {
                code: payload.code.ok_or_else(|| bad_request("invalid browse cursor"))?,
                id: payload.id.ok_or_else(|| bad_request("invalid browse cursor"))?,
            })
        })
        .transpose()?;
    let (rows, next) =
        selection_repo::list_offerings(&state.db, &filter, cursor.as_ref(), params.limit).await?;
    let next_cursor = next
        .map(|cursor| {
            encode_cursor(&OfferingCursorPayload {
                version: 1,
                mode: "browse".into(),
                fingerprint,
                code: Some(cursor.code),
                id: Some(cursor.id),
                offset: None,
            })
        })
        .transpose()?;
    Ok(Page::new(rows.into_iter().map(row_to_course_dto).collect(), next_cursor))
}

async fn check_search_rate_limit(
    state: &AppState,
    headers: &HeaderMap,
    params: &OfferingsQuery,
) -> AppResult<()> {
    if normalized_query(params).is_some() {
        shared::ratelimit::check_token_bucket(
            state.redis.as_ref(),
            "selection-search",
            &rate_limit_key(headers),
            30,
            10,
        )
        .await?;
    }
    Ok(())
}

pub async fn selection_offerings(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: Result<Query<OfferingsQuery>, QueryRejection>,
) -> AppResult<Json<Page<SelectionCourseDto>>> {
    let params = parse_selection_query(query)?;
    validate_offerings_query(&params)?;
    check_search_rate_limit(&state, &headers, &params).await?;
    let cursor_key = params.cursor.as_deref().unwrap_or("first");
    let cache_key = versioned_cache_id(
        &state,
        "selection-offerings",
        &format!("{}:{}:{}", filter_fingerprint(&params), cursor_key, params.limit),
    )
    .await;
    let page = shared::cache::cached_json(
        state.redis.as_ref(),
        "selection-offerings",
        &cache_key,
        60,
        fetch_offering_page(&state, &params),
    )
    .await?;
    Ok(Json(page))
}

pub async fn selection_offering(
    State(state): State<AppState>,
    Path(raw_id): Path<String>,
) -> AppResult<Json<SelectionCourseDto>> {
    let offering_id = parse_positive_id(&raw_id, "offeringId")?;
    let key =
        versioned_cache_id(&state, "selection-offering-detail", &offering_id.to_string()).await;
    let item = shared::cache::cached_json(
        state.redis.as_ref(),
        "selection-offering-detail",
        &key,
        300,
        async {
            selection_repo::find_offering_by_id(&state.db, offering_id)
                .await?
                .map(row_to_course_dto)
                .ok_or(CoursesError::SelectionCourseNotFound)
        },
    )
    .await?;
    Ok(Json(item))
}

async fn offering_timeslots(state: &AppState, offering_id: i64) -> AppResult<Vec<TimeSlotDto>> {
    if selection_repo::find_offering_by_id(&state.db, offering_id).await?.is_none() {
        return Err(CoursesError::SelectionCourseNotFound.into());
    }
    Ok(selection_repo::list_timeslots(&state.db, offering_id)
        .await?
        .into_iter()
        .map(row_to_timeslot_dto)
        .collect())
}

fn parse_positive_id(raw_id: &str, field: &str) -> AppResult<i64> {
    let value = raw_id.parse::<i64>().map_err(|_| bad_request(format!("invalid {field}")))?;
    if value <= 0 {
        return Err(bad_request(format!("invalid {field}")));
    }
    Ok(value)
}

pub async fn selection_offering_timeslots(
    State(state): State<AppState>,
    Path(raw_id): Path<String>,
) -> AppResult<Json<Vec<TimeSlotDto>>> {
    let offering_id = parse_positive_id(&raw_id, "offeringId")?;
    let key =
        versioned_cache_id(&state, "selection-offering-timeslots", &offering_id.to_string()).await;
    let items = shared::cache::cached_json(
        state.redis.as_ref(),
        "selection-offering-timeslots",
        &key,
        300,
        offering_timeslots(&state, offering_id),
    )
    .await?;
    Ok(Json(items))
}

// ---- bounded compatibility adapters ----

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoursesByMajorQuery {
    pub calendar_id: i64,
    pub major_id: i64,
    pub grade: String,
}

pub async fn selection_courses_by_major(
    State(state): State<AppState>,
    query: Result<Query<CoursesByMajorQuery>, QueryRejection>,
) -> AppResult<Json<Vec<SelectionCourseDto>>> {
    let params = parse_selection_query(query)?;
    validate_calendar_id(params.calendar_id)?;
    if params.major_id <= 0 {
        return Err(bad_request("majorId must be positive"));
    }
    let grade = params.grade.trim();
    if grade.is_empty() || grade.chars().count() > 16 || grade.chars().any(char::is_control) {
        return Err(bad_request("grade must contain 1-16 non-control characters"));
    }
    let rows = selection_repo::list_courses_by_major(
        &state.db,
        params.calendar_id,
        params.major_id,
        grade,
    )
    .await?;
    Ok(Json(rows.into_iter().map(row_to_course_dto).collect()))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoursesByNatureQuery {
    pub calendar_id: i64,
    pub nature_id: i64,
}

pub async fn selection_courses_by_nature(
    State(state): State<AppState>,
    query: Result<Query<CoursesByNatureQuery>, QueryRejection>,
) -> AppResult<Json<Vec<SelectionCourseDto>>> {
    let params = parse_selection_query(query)?;
    validate_calendar_id(params.calendar_id)?;
    if params.nature_id <= 0 {
        return Err(bad_request("natureId must be positive"));
    }
    let rows =
        selection_repo::list_courses_by_nature(&state.db, params.calendar_id, params.nature_id)
            .await?;
    Ok(Json(rows.into_iter().map(row_to_course_dto).collect()))
}

pub async fn selection_course_by_id(
    State(state): State<AppState>,
    Path(raw_id): Path<String>,
) -> AppResult<Json<SelectionCourseDto>> {
    let teaching_class_id = parse_positive_id(&raw_id, "teachingClassId")?;
    let row = selection_repo::find_selection_course_by_id(&state.db, teaching_class_id)
        .await?
        .ok_or(CoursesError::SelectionCourseNotFound)?;
    Ok(Json(row_to_course_dto(row)))
}

#[derive(Debug, Deserialize)]
pub struct SelectionSearchQuery {
    #[serde(rename = "calendarId")]
    pub calendar_id: i64,
    pub q: String,
}

pub async fn selection_courses_search(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: Result<Query<SelectionSearchQuery>, QueryRejection>,
) -> AppResult<Json<Vec<SelectionCourseDto>>> {
    let params = parse_selection_query(query)?;
    validate_calendar_id(params.calendar_id)?;
    let offering_params = OfferingsQuery {
        q: Some(params.q),
        calendar_id: params.calendar_id,
        major_id: None,
        grade: None,
        nature_id: None,
        campus_id: None,
        course_code: None,
        weekday: None,
        start_slot: None,
        end_slot: None,
        week: None,
        include_unknown_schedule: true,
        cursor: None,
        limit: 20,
    };
    validate_offerings_query(&offering_params)?;
    check_search_rate_limit(&state, &headers, &offering_params).await?;
    let page = fetch_offering_page(&state, &offering_params).await?;
    Ok(Json(page.items))
}

pub async fn selection_course_timeslots(
    State(state): State<AppState>,
    Path(raw_id): Path<String>,
) -> AppResult<Json<Vec<TimeSlotDto>>> {
    let teaching_class_id = parse_positive_id(&raw_id, "teachingClassId")?;
    let course = selection_repo::find_selection_course_by_id(&state.db, teaching_class_id)
        .await?
        .ok_or(CoursesError::SelectionCourseNotFound)?;
    Ok(Json(offering_timeslots(&state, course.id).await?))
}

pub async fn selection_latest_update(
    State(state): State<AppState>,
) -> AppResult<Json<LatestUpdateDto>> {
    let key = versioned_cache_id(&state, "selection-latest-update", "all").await;
    let dto = shared::cache::cached_json(
        state.redis.as_ref(),
        "selection-latest-update",
        &key,
        60,
        async {
            let freshness = selection_repo::find_latest_update(&state.db).await?;
            let stale = freshness.updated_at.is_none_or(|updated_at| {
                Utc::now().signed_duration_since(updated_at)
                    > Duration::hours(SELECTION_STALE_AFTER_HOURS)
            });
            Ok::<_, CoursesError>(LatestUpdateDto {
                updated_at: freshness.updated_at.map(|value| value.to_rfc3339()),
                imported_at: freshness.imported_at.map(|value| value.to_rfc3339()),
                stale,
                stale_after_hours: SELECTION_STALE_AFTER_HOURS as i32,
            })
        },
    )
    .await?;
    Ok(Json(dto))
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderMap;

    use super::{
        next_search_batch_size, normalized_query, rate_limit_key, validate_offerings_query,
        OfferingsQuery, MAX_SEARCH_BATCHES_PER_PAGE, MAX_SEARCH_WINDOW,
    };

    fn offerings_query(calendar_id: i64) -> OfferingsQuery {
        OfferingsQuery {
            q: None,
            calendar_id,
            major_id: None,
            grade: None,
            nature_id: None,
            campus_id: None,
            course_code: None,
            weekday: None,
            start_slot: None,
            end_slot: None,
            week: None,
            include_unknown_schedule: true,
            cursor: None,
            limit: 20,
        }
    }

    #[test]
    fn stale_candidate_scan_is_bounded_to_five_batches() {
        let mut search_offset = 0;
        let mut completed_batches = 0;
        while let Some(batch_size) = next_search_batch_size(search_offset, completed_batches) {
            search_offset += batch_size;
            completed_batches += 1;
        }

        assert_eq!(completed_batches, MAX_SEARCH_BATCHES_PER_PAGE);
        assert_eq!(search_offset, MAX_SEARCH_WINDOW);
    }

    #[test]
    fn canonical_offerings_require_a_positive_calendar() {
        assert!(validate_offerings_query(&offerings_query(0)).is_err());
        assert!(validate_offerings_query(&offerings_query(1)).is_ok());
    }

    #[test]
    fn canonical_offerings_reject_blank_filters_and_unscoped_strict_time() {
        let mut query = offerings_query(1);
        query.q = Some("  ".into());
        assert!(validate_offerings_query(&query).is_err());

        query.q = None;
        query.grade = Some("\t".into());
        assert!(validate_offerings_query(&query).is_err());

        query.grade = None;
        query.include_unknown_schedule = false;
        assert!(validate_offerings_query(&query).is_err());
    }

    #[test]
    fn only_non_blank_search_uses_the_stable_source_rate_limit_key() {
        let mut browse = offerings_query(1);
        assert!(normalized_query(&browse).is_none());

        browse.q = Some("course".into());
        assert!(normalized_query(&browse).is_some());
        let first = HeaderMap::from_iter([(
            "x-forwarded-for".parse().expect("header name"),
            "203.0.113.10, 10.0.0.1".parse().expect("header value"),
        )]);
        let second = HeaderMap::from_iter([(
            "x-forwarded-for".parse().expect("header name"),
            "203.0.113.10".parse().expect("header value"),
        )]);
        assert_eq!(rate_limit_key(&first), rate_limit_key(&second));
    }
}
