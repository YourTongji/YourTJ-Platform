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

/// Require an independent moderator whose persisted role is strictly above the uploader's role.
pub(crate) fn require_strictly_lower_owner(
    auth: &AuthAccount,
    owner_id: i64,
    owner_role: &str,
) -> AppResult<()> {
    let actor_rank = role_rank(&auth.role).ok_or(AppError::Forbidden)?;
    let owner_rank = role_rank(owner_role).ok_or(AppError::Forbidden)?;
    if auth.id == owner_id || actor_rank <= owner_rank {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use shared::AuthAccount;

    use super::require_strictly_lower_owner;

    fn account(id: i64, role: &str) -> AuthAccount {
        AuthAccount { id, role: role.into(), status: "active".into() }
    }

    #[test]
    fn moderation_requires_independence_and_strict_role_hierarchy() {
        assert!(require_strictly_lower_owner(&account(2, "mod"), 1, "user").is_ok());
        assert!(require_strictly_lower_owner(&account(3, "admin"), 2, "mod").is_ok());
        assert!(require_strictly_lower_owner(&account(3, "admin"), 1, "user").is_ok());
        assert!(require_strictly_lower_owner(&account(1, "admin"), 1, "user").is_err());
        assert!(require_strictly_lower_owner(&account(2, "mod"), 3, "mod").is_err());
        assert!(require_strictly_lower_owner(&account(2, "mod"), 3, "admin").is_err());
    }
}
