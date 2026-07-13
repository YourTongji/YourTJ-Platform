//! System moderation policy for verified upload callbacks.

use shared::{AppError, AppResult};
use sqlx::PgConnection;

const AUTO_APPROVAL_POLICY_VERSION: i32 = 1;

fn is_auto_approval_eligible(enabled: bool, kind: &str, mime: &str) -> bool {
    enabled && kind == "image" && matches!(mime, "image/jpeg" | "image/png" | "image/webp")
}

/// Apply the narrow system image policy after OSS callback evidence has been verified.
///
/// This changes only the moderation state. Publication remains fail-closed until the durable
/// worker verifies and re-encodes every Delivery variant.
pub(crate) async fn apply_callback_policy(
    connection: &mut PgConnection,
    upload_id: i64,
    kind: &str,
    mime: &str,
    is_enabled: bool,
) -> AppResult<bool> {
    if !is_auto_approval_eligible(is_enabled, kind, mime) {
        return Ok(false);
    }
    let affected = sqlx::query(
        "UPDATE media.uploads SET status = 'clean' \
         WHERE id = $1 AND status = 'pending' AND kind = 'image' \
           AND mime IN ('image/jpeg', 'image/png', 'image/webp')",
    )
    .bind(upload_id)
    .execute(&mut *connection)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(AppError::Conflict("upload is no longer eligible for auto-approval".into()));
    }
    crate::processing::enqueue_variant_processing(&mut *connection, upload_id).await?;
    governance::record_system_event_tx(
        &mut *connection,
        "media.upload.auto_approved",
        "upload",
        &upload_id.to_string(),
        "verified raster upload accepted by the active automatic moderation policy",
        Some(&serde_json::json!({
            "policyVersion": AUTO_APPROVAL_POLICY_VERSION,
            "oldStatus": "pending",
            "newStatus": "clean",
            "decisionSource": "system_policy",
            "contentReview": "not_performed",
            "publicationState": "processing",
        })),
    )
    .await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::is_auto_approval_eligible;

    #[test]
    fn auto_approval_is_limited_to_enabled_supported_raster_images() {
        for mime in ["image/jpeg", "image/png", "image/webp"] {
            assert!(is_auto_approval_eligible(true, "image", mime));
            assert!(!is_auto_approval_eligible(false, "image", mime));
        }
        assert!(!is_auto_approval_eligible(true, "file", "application/pdf"));
        assert!(!is_auto_approval_eligible(true, "image", "image/gif"));
        assert!(!is_auto_approval_eligible(true, "image", "image/svg+xml"));
    }
}
