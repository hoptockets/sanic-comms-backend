use std::collections::{HashMap, HashSet};

use revolt_database::User;
use revolt_models::v0::PlatformAdminRole;
use revolt_result::{create_error, Result};

pub const PERM_REPORTS_READ: &str = "reports.read";
pub const PERM_REPORTS_REVIEW: &str = "reports.review";
pub const PERM_REPORTS_ASSIGN: &str = "reports.assign";
pub const PERM_REPORTS_ESCALATE: &str = "reports.escalate";
pub const PERM_REPORTS_RESOLVE: &str = "reports.resolve";
pub const PERM_REPORTS_REJECT: &str = "reports.reject";
pub const PERM_REPORTS_TIMELINE_READ: &str = "reports.timeline.read";
pub const PERM_REPORTS_ACTIONS_WRITE: &str = "reports.actions.write";
pub const PERM_USERS_READ: &str = "users.read";
pub const PERM_USERS_FLAGS_WRITE: &str = "users.flags.write";
pub const PERM_USERS_COSMETICS_WRITE: &str = "users.cosmetics.write";
pub const PERM_USERS_RESTRICTIONS_WRITE: &str = "users.restrictions.write";
pub const PERM_USERS_RECOVERY_TRIGGER: &str = "users.recovery.trigger";
pub const PERM_STAFF_READ: &str = "staff.read";
pub const PERM_STAFF_ROLE_MANAGE: &str = "staff.role.manage";
pub const PERM_STAFF_PERMISSIONS_MANAGE: &str = "staff.permissions.manage";
pub const PERM_SYSTEM_FEATURES_WRITE: &str = "system.features.write";
pub const PERM_SYSTEM_KILL_SWITCH_WRITE: &str = "system.kill_switch.write";
pub const PERM_SYSTEM_EMAIL_WRITE: &str = "system.email.write";
pub const PERM_AUDIT_READ: &str = "audit.read";
pub const PERM_POLICY_READ: &str = "policy.read";

pub const PERMISSION_CATALOG: [&str; 22] = [
    PERM_REPORTS_READ,
    PERM_REPORTS_REVIEW,
    PERM_REPORTS_ASSIGN,
    PERM_REPORTS_ESCALATE,
    PERM_REPORTS_RESOLVE,
    PERM_REPORTS_REJECT,
    PERM_REPORTS_TIMELINE_READ,
    PERM_REPORTS_ACTIONS_WRITE,
    PERM_USERS_READ,
    PERM_USERS_FLAGS_WRITE,
    PERM_USERS_COSMETICS_WRITE,
    PERM_USERS_RESTRICTIONS_WRITE,
    PERM_USERS_RECOVERY_TRIGGER,
    PERM_STAFF_READ,
    PERM_STAFF_ROLE_MANAGE,
    PERM_STAFF_PERMISSIONS_MANAGE,
    PERM_SYSTEM_FEATURES_WRITE,
    PERM_SYSTEM_KILL_SWITCH_WRITE,
    PERM_SYSTEM_EMAIL_WRITE,
    PERM_AUDIT_READ,
    PERM_POLICY_READ,
    "feature:*",
];

pub fn role_templates() -> HashMap<String, Vec<String>> {
    HashMap::from([
        (
            "PlatformOwner".to_owned(),
            PERMISSION_CATALOG
                .iter()
                .copied()
                .filter(|value| *value != "feature:*")
                .map(ToString::to_string)
                .collect(),
        ),
        (
            "SafetyAdmin".to_owned(),
            vec![
                PERM_REPORTS_READ,
                PERM_REPORTS_REVIEW,
                PERM_REPORTS_ASSIGN,
                PERM_REPORTS_ESCALATE,
                PERM_REPORTS_RESOLVE,
                PERM_REPORTS_REJECT,
                PERM_REPORTS_TIMELINE_READ,
                PERM_REPORTS_ACTIONS_WRITE,
                PERM_USERS_READ,
                PERM_USERS_FLAGS_WRITE,
                PERM_USERS_COSMETICS_WRITE,
                PERM_USERS_RESTRICTIONS_WRITE,
                PERM_STAFF_READ,
                PERM_AUDIT_READ,
                PERM_POLICY_READ,
            ]
            .into_iter()
            .map(ToString::to_string)
            .collect(),
        ),
        (
            "SupportAgent".to_owned(),
            vec![
                PERM_REPORTS_READ,
                PERM_REPORTS_REVIEW,
                PERM_REPORTS_ASSIGN,
                PERM_REPORTS_RESOLVE,
                PERM_REPORTS_REJECT,
                PERM_REPORTS_TIMELINE_READ,
                PERM_USERS_READ,
                PERM_USERS_FLAGS_WRITE,
                PERM_USERS_RECOVERY_TRIGGER,
                PERM_AUDIT_READ,
                PERM_POLICY_READ,
            ]
            .into_iter()
            .map(ToString::to_string)
            .collect(),
        ),
        (
            "Analyst".to_owned(),
            vec![
                PERM_REPORTS_READ,
                PERM_REPORTS_TIMELINE_READ,
                PERM_USERS_READ,
                PERM_STAFF_READ,
                PERM_AUDIT_READ,
                PERM_POLICY_READ,
            ]
            .into_iter()
            .map(ToString::to_string)
            .collect(),
        ),
    ])
}

pub fn validate_permissions(entries: &[String]) -> Result<()> {
    for entry in entries {
        if entry.starts_with("feature:") {
            continue;
        }

        if !PERMISSION_CATALOG.iter().any(|known| known == &entry.as_str()) {
            return Err(create_error!(FailedValidation {
                error: format!("Unknown platform permission: {entry}")
            }));
        }
    }
    Ok(())
}

fn effective_permission_set(user: &User) -> HashSet<String> {
    let mut set = HashSet::new();

    if let Some(role) = user.platform_admin_role.as_ref() {
        if let Some(defaults) = role_templates().get(&format!("{role:?}")) {
            set.extend(defaults.iter().cloned());
        }
    }

    if let Some(explicit) = user.platform_permissions.as_ref() {
        set.extend(explicit.iter().cloned());
    }

    set
}

pub fn has_permission(user: &User, permission: &str) -> bool {
    if user.privileged {
        return true;
    }

    if matches!(user.platform_admin_role, Some(PlatformAdminRole::PlatformOwner)) {
        return true;
    }

    let set = effective_permission_set(user);
    if set.contains(permission) {
        return true;
    }

    if permission.starts_with("feature:") && set.contains("feature:*") {
        return true;
    }

    false
}

pub fn ensure_permission(user: &User, permission: &str) -> Result<()> {
    if has_permission(user, permission) {
        Ok(())
    } else {
        Err(create_error!(NotPrivileged))
    }
}

pub fn ensure_platform_owner(user: &User) -> Result<()> {
    if user.privileged || matches!(user.platform_admin_role, Some(PlatformAdminRole::PlatformOwner))
    {
        Ok(())
    } else {
        Err(create_error!(NotPrivileged))
    }
}

pub fn ensure_owner_delegation(user: &User, permissions: &[String]) -> Result<()> {
    ensure_platform_owner(user)?;
    validate_permissions(permissions)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_known_permissions() {
        let values = vec![
            PERM_REPORTS_READ.to_owned(),
            PERM_USERS_FLAGS_WRITE.to_owned(),
            "feature:profile_v3".to_owned(),
        ];
        assert!(validate_permissions(&values).is_ok());
    }

    #[test]
    fn rejects_unknown_permissions() {
        let values = vec!["unknown.permission".to_owned()];
        assert!(validate_permissions(&values).is_err());
    }

    #[test]
    fn role_templates_include_owner() {
        let templates = role_templates();
        assert!(templates.contains_key("PlatformOwner"));
        assert!(
            templates
                .get("PlatformOwner")
                .map(|values| values.contains(&PERM_SYSTEM_KILL_SWITCH_WRITE.to_owned()))
                .unwrap_or(false)
        );
    }
}

