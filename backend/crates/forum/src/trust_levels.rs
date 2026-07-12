//! Trust-level reads used by forum permission and rate-limit checks.
//!
//! The activity domain owns evaluation, history, and the identity projection.
//! Forum only reads the effective level for board gates, DM access, and flag weight.

use sqlx::PgPool;

use shared::AppResult;

/// Read the authoritative trust level used for posting and interaction policy.
pub async fn get_trust_level(pool: &PgPool, account_id: i64) -> AppResult<i16> {
    activity::trust::get_trust_level(pool, account_id).await
}

/// Report flag weight for the unified 1–6 trust scale.
pub fn flag_weight(trust_level: i16) -> f32 {
    match trust_level {
        1 => 0.5,
        2 => 1.0,
        3 => 1.5,
        4..=6 => 2.0,
        // Unregistered / unexpected values stay conservative.
        _ => 0.5,
    }
}

/// Apply automatic one-step upgrades for active accounts.
pub async fn run_daily_tl_promotion(pool: &PgPool) -> (i64, i64) {
    activity::trust::run_scheduled_trust_evaluation(pool).await
}

#[cfg(test)]
mod tests {
    use super::flag_weight;

    #[test]
    fn report_weights_match_unified_trust_scale() {
        assert_eq!(flag_weight(1), 0.5);
        assert_eq!(flag_weight(2), 1.0);
        assert_eq!(flag_weight(3), 1.5);
        assert_eq!(flag_weight(4), 2.0);
        assert_eq!(flag_weight(5), 2.0);
        assert_eq!(flag_weight(6), 2.0);
        // Unregistered or out-of-range values are conservative (same as Lv.1).
        assert_eq!(flag_weight(0), 0.5);
        assert_eq!(flag_weight(-1), 0.5);
        assert_eq!(flag_weight(99), 0.5);
    }
}
