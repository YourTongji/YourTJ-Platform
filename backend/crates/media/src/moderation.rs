//! Authorization rules shared by every media moderation surface.

use shared::{AppError, AppResult, AuthAccount};

fn role_rank(role: &str) -> Option<u8> {
    match role {
        "user" => Some(0),
        "mod" => Some(1),
        "admin" => Some(2),
        _ => None,
    }
}

/// The persisted authorization outcome carried into preview evidence and governance audit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModerationAuthorization {
    pub is_self_review: bool,
}

/// Authorize an independent higher-role moderator or the narrow ADMIN own-media exception.
pub(crate) fn authorize_moderation(
    auth: &AuthAccount,
    owner_id: i64,
    owner_role: &str,
    self_review_confirmed: bool,
) -> AppResult<ModerationAuthorization> {
    let actor_rank = role_rank(&auth.role).ok_or(AppError::Forbidden)?;
    let owner_rank = role_rank(owner_role).ok_or(AppError::Forbidden)?;
    if auth.id == owner_id {
        if auth.role == "admin" && owner_role == "admin" && self_review_confirmed {
            return Ok(ModerationAuthorization { is_self_review: true });
        }
        return Err(AppError::Forbidden);
    }
    if actor_rank <= owner_rank {
        return Err(AppError::Forbidden);
    }
    Ok(ModerationAuthorization { is_self_review: false })
}

#[cfg(test)]
mod tests {
    use shared::AuthAccount;

    use super::authorize_moderation;

    fn account(id: i64, role: &str) -> AuthAccount {
        AuthAccount { id, role: role.into(), status: "active".into() }
    }

    #[test]
    fn independent_moderation_still_requires_strict_role_hierarchy() {
        assert!(
            !authorize_moderation(&account(2, "mod"), 1, "user", false)
                .expect("moderator over user")
                .is_self_review
        );
        assert!(
            !authorize_moderation(&account(3, "admin"), 2, "mod", false)
                .expect("admin over moderator")
                .is_self_review
        );
        assert!(authorize_moderation(&account(2, "mod"), 3, "mod", false).is_err());
        assert!(authorize_moderation(&account(2, "mod"), 3, "admin", false).is_err());
        assert!(authorize_moderation(&account(2, "admin"), 3, "admin", false).is_err());
    }

    #[test]
    fn only_admin_can_explicitly_confirm_own_media_review() {
        assert!(authorize_moderation(&account(1, "admin"), 1, "admin", false).is_err());
        assert!(authorize_moderation(&account(1, "mod"), 1, "mod", true).is_err());
        assert!(authorize_moderation(&account(1, "user"), 1, "user", true).is_err());
        assert!(authorize_moderation(&account(1, "admin"), 1, "user", true).is_err());
        assert!(
            authorize_moderation(&account(1, "admin"), 1, "admin", true)
                .expect("explicit ADMIN own-media exception")
                .is_self_review
        );
    }
}
