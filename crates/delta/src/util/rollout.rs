use std::collections::HashMap;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Mutex};

use once_cell::sync::Lazy;
use revolt_database::User;
use revolt_models::v0::PlatformAdminRole;

static GLOBAL_FEATURE_OVERRIDES: Lazy<Mutex<HashMap<String, bool>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static EMERGENCY_KILL_SWITCH: Lazy<AtomicBool> = Lazy::new(|| {
    AtomicBool::new(std::env::var("COMMS_FEATURE_EMERGENCY_KILL").unwrap_or_default() == "1")
});

fn parse_csv_env(name: &str) -> Vec<String> {
    std::env::var(name)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

pub fn default_features() -> HashMap<String, bool> {
    HashMap::from([
        ("profile_v2".to_string(), true),
        ("profile_v3".to_string(), true),
        ("admin_panel_v1".to_string(), true),
        ("admin_reports_v2".to_string(), true),
        ("staff_permissions_v2".to_string(), true),
        ("user_cosmetics_admin_v1".to_string(), true),
        ("server_theming_v1".to_string(), true),
        ("stickers_v1".to_string(), true),
        ("soundboard_v1".to_string(), true),
    ])
}

pub fn global_features() -> HashMap<String, bool> {
    let mut map = default_features();

    for key in parse_csv_env("COMMS_FEATURE_GLOBAL_DISABLE") {
        map.insert(key, false);
    }

    if let Ok(overrides) = GLOBAL_FEATURE_OVERRIDES.lock() {
        for (key, value) in overrides.iter() {
            map.insert(key.clone(), *value);
        }
    }

    map
}

pub fn promoted_features(user: &User) -> Vec<String> {
    let mut out: Vec<String> = user
        .platform_permissions
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter_map(|value| value.strip_prefix("feature:"))
        .map(ToString::to_string)
        .collect();

    let cohort: Vec<String> = parse_csv_env("COMMS_FEATURE_PROMOTE_USER_IDS");
    if cohort.iter().any(|id| id == &user.id) {
        for key in parse_csv_env("COMMS_FEATURE_PROMOTE_KEYS") {
            if !out.contains(&key) {
                out.push(key);
            }
        }
    }

    out
}

/// Whether a feature is enabled for this user after kill-switch, globals, and promotions.
pub fn user_has_feature(user: &User, key: &str) -> bool {
    if emergency_kill_switch() {
        return false;
    }
    effective_features(user)
        .get(key)
        .copied()
        .unwrap_or(false)
}

pub fn effective_features(user: &User) -> HashMap<String, bool> {
    let mut effective = global_features();
    for key in promoted_features(user) {
        effective.insert(key, true);
    }

    let staff_admin = user.privileged
        || matches!(
            user.platform_admin_role,
            Some(PlatformAdminRole::PlatformOwner)
                | Some(PlatformAdminRole::SafetyAdmin)
                | Some(PlatformAdminRole::SupportAgent)
                | Some(PlatformAdminRole::Analyst)
        );
    if staff_admin {
        effective.insert("admin_panel_v1".to_string(), true);
        effective.insert("profile_v2".to_string(), true);
        effective.insert("profile_v3".to_string(), true);
        effective.insert("server_theming_v1".to_string(), true);
    }

    effective
}

pub fn emergency_kill_switch() -> bool {
    EMERGENCY_KILL_SWITCH.load(Ordering::Relaxed)
}

pub fn set_emergency_kill_switch(enabled: bool) {
    EMERGENCY_KILL_SWITCH.store(enabled, Ordering::Relaxed);
}

pub fn set_global_feature(feature: String, enabled: bool) {
    if let Ok(mut overrides) = GLOBAL_FEATURE_OVERRIDES.lock() {
        overrides.insert(feature, enabled);
    }
}
