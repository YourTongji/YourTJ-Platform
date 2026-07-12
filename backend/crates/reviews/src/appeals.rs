//! Owner-domain validation and reversal for review moderation appeals.

use shared::{AppError, AppResult};
use sqlx::PgConnection;

#[derive(Debug, Clone)]
pub struct ReviewAppealTarget {
    pub review_id: i64,
    pub author_role: String,
    pub restore_status: String,
}

#[derive(Debug, Clone)]
pub struct ReviewAppealMutation {
    pub review_id: i64,
    pub course_id: i64,
}

fn restore_status_from_metadata(
    action: &str,
    metadata: Option<&serde_json::Value>,
) -> AppResult<String> {
    let metadata = metadata.ok_or(AppError::NotFound)?;
    let old_status = metadata
        .get("oldStatus")
        .and_then(serde_json::Value::as_str)
        .filter(|status| matches!(*status, "visible" | "pending"))
        .ok_or(AppError::NotFound)?;
    if metadata.get("newStatus").and_then(serde_json::Value::as_str) != Some("hidden") {
        return Err(AppError::NotFound);
    }
    if action == "reviews.report.decided"
        && (metadata.get("decision").and_then(serde_json::Value::as_str) != Some("upheld")
            || metadata.get("contentChanged").and_then(serde_json::Value::as_bool) != Some(true))
    {
        return Err(AppError::NotFound);
    }
    Ok(old_status.to_owned())
}

async fn review_target_for_event(
    connection: &mut PgConnection,
    action: &str,
    target_type: &str,
    target_id: &str,
    metadata: Option<&serde_json::Value>,
) -> AppResult<(i64, String)> {
    let restore_status = restore_status_from_metadata(action, metadata)?;
    if action == "reviews.review.hidden" && target_type == "review" {
        return Ok((target_id.parse().map_err(|_| AppError::NotFound)?, restore_status));
    }
    if action == "reviews.report.decided" && target_type == "review_report" {
        let report_id: i64 = target_id.parse().map_err(|_| AppError::NotFound)?;
        let review_id = sqlx::query_scalar(
            "SELECT review_id FROM reviews.review_reports \
             WHERE id = $1 AND status = 'upheld' FOR SHARE",
        )
        .bind(report_id)
        .fetch_optional(connection)
        .await?
        .ok_or(AppError::NotFound)?;
        return Ok((review_id, restore_status));
    }
    Err(AppError::NotFound)
}

/// Validate a hidden review as an appeal target owned by the appellant.
pub async fn inspect_appealable_review_tx(
    connection: &mut PgConnection,
    action: &str,
    target_type: &str,
    target_id: &str,
    metadata: Option<&serde_json::Value>,
    appellant_account_id: i64,
) -> AppResult<ReviewAppealTarget> {
    let (review_id, restore_status) =
        review_target_for_event(connection, action, target_type, target_id, metadata).await?;
    let review: (Option<i64>, String) = sqlx::query_as(
        "SELECT account_id, status::text FROM reviews.reviews WHERE id = $1 FOR SHARE",
    )
    .bind(review_id)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or(AppError::NotFound)?;
    if review.0 != Some(appellant_account_id) {
        return Err(AppError::NotFound);
    }
    if review.1 != "hidden" {
        return Err(AppError::Conflict("the review disposition is no longer active".into()));
    }
    let author_role =
        identity::public_accounts::find_account_role_by_id(connection, appellant_account_id)
            .await?
            .ok_or(AppError::NotFound)?;
    Ok(ReviewAppealTarget { review_id, author_role, restore_status })
}

/// Restore the exact hidden review under appeal and its aggregate/activity projections.
#[allow(clippy::too_many_arguments)] // reason: original target keys are required for the fail-closed later-event guard
pub async fn overturn_review_for_appeal_tx(
    connection: &mut PgConnection,
    original_event_id: i64,
    original_action: &str,
    original_target_type: &str,
    original_target_id: &str,
    original_metadata: Option<&serde_json::Value>,
    review_id: i64,
    appellant_account_id: i64,
) -> AppResult<ReviewAppealMutation> {
    let (event_review_id, restore_status) = review_target_for_event(
        connection,
        original_action,
        original_target_type,
        original_target_id,
        original_metadata,
    )
    .await?;
    if event_review_id != review_id {
        return Err(AppError::NotFound);
    }
    if governance::has_later_target_event_tx(
        connection,
        original_event_id,
        original_target_type,
        original_target_id,
        Some("review"),
        Some(&review_id.to_string()),
    )
    .await?
    {
        return Err(AppError::Conflict(
            "review state changed after the appealed disposition".into(),
        ));
    }
    let review = sqlx::query_as::<_, crate::models::ReviewRow>(
        "SELECT id, course_id, account_id, rating, comment, score, semester, \
                approve_count, disapprove_count, status::text, created_at, updated_at, \
                reviewer_name, reviewer_avatar \
         FROM reviews.reviews WHERE id = $1 FOR UPDATE",
    )
    .bind(review_id)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or(AppError::NotFound)?;
    if review.account_id != Some(appellant_account_id) {
        return Err(AppError::NotFound);
    }
    if review.status != "hidden" {
        return Err(AppError::Conflict(
            "review state no longer matches the appealed disposition".into(),
        ));
    }
    crate::repo::set_review_status_tx(connection, &review, &restore_status).await?;
    Ok(ReviewAppealMutation { review_id, course_id: review.course_id })
}

#[cfg(test)]
mod tests {
    use super::restore_status_from_metadata;

    #[test]
    fn direct_hide_restores_the_exact_previous_status() {
        let metadata = serde_json::json!({ "oldStatus": "pending", "newStatus": "hidden" });
        assert_eq!(
            restore_status_from_metadata("reviews.review.hidden", Some(&metadata))
                .expect("valid direct restriction"),
            "pending"
        );
    }

    #[test]
    fn report_without_a_real_visibility_change_is_not_appealable() {
        let metadata = serde_json::json!({
            "decision": "upheld",
            "oldStatus": "hidden",
            "newStatus": "hidden",
            "contentChanged": false
        });
        assert!(restore_status_from_metadata("reviews.report.decided", Some(&metadata)).is_err());
    }
}
