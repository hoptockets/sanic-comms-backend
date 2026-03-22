use iso8601_timestamp::Timestamp;
use once_cell::sync::Lazy;
use revolt_config::config;
use revolt_database::{Database, PartialUser, Report, User, UserSettings};
use revolt_models::v0::{
    PlatformAdminRole, ReportPriority, ReportSeverity, ReportStatus, ReportStatusString,
};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::util::{admin_permissions, audit, rollout};

#[derive(Deserialize, rocket::form::FromForm, JsonSchema)]
pub struct ReportQueueQuery {
    /// Optional moderation queue status filter.
    ///
    /// We intentionally keep this as a `String` for Rocket query parsing, and convert
    /// to `ReportStatusString` ourselves.
    pub status: Option<String>,
    pub severity: Option<String>,
    pub priority: Option<String>,
    pub assignee_id: Option<String>,
    pub reviewer_id: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub sort: Option<String>,
}

#[derive(Serialize, JsonSchema)]
pub struct ReportQueueResponse {
    pub reports: Vec<revolt_models::v0::Report>,
    pub total: usize,
}

#[derive(Serialize, JsonSchema)]
pub struct ReportDashboardResponse {
    pub total: usize,
    pub created: usize,
    pub in_review: usize,
    pub escalated: usize,
    pub resolved: usize,
    pub rejected: usize,
}

#[derive(Deserialize, JsonSchema)]
pub struct DataReviewReport {
    pub status: ReportStatusString,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub assignee_id: Option<String>,
    #[serde(default)]
    pub rejection_reason: Option<String>,
    #[serde(default)]
    pub resolution_reason: Option<String>,
}

#[derive(Serialize, JsonSchema)]
pub struct ReportTimelineResponse {
    pub report_id: String,
    pub entries: Vec<audit::AuditEntry>,
}

#[derive(Serialize, JsonSchema)]
pub struct ReportSnapshotsResponse {
    pub report_id: String,
    pub snapshots: Vec<serde_json::Value>,
}

#[derive(Deserialize, JsonSchema)]
pub struct DataReportAction {
    pub action: String,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub assignee_id: Option<String>,
    #[serde(default)]
    pub merge_into: Option<String>,
}

#[derive(Serialize, JsonSchema)]
pub struct UserInspectResponse {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    pub display_name: Option<String>,
    pub privileged: bool,
    pub platform_admin_role: Option<PlatformAdminRole>,
    pub platform_permissions: Vec<String>,
    pub badges: u32,
    pub flags: u32,
}

#[derive(Deserialize, rocket::form::FromForm, JsonSchema)]
pub struct UsersListQuery {
    pub q: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Serialize, JsonSchema)]
pub struct UsersListResponse {
    pub users: Vec<UserInspectResponse>,
}

#[derive(Deserialize, rocket::form::FromForm, JsonSchema)]
pub struct StaffListQuery {
    pub ids: Option<String>,
    pub q: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Serialize, JsonSchema)]
pub struct StaffListResponse {
    pub users: Vec<UserInspectResponse>,
}

#[derive(Serialize, JsonSchema)]
pub struct StaffPermissionsCatalogResponse {
    pub catalog: Vec<String>,
    pub role_templates: HashMap<String, Vec<String>>,
}

#[derive(Deserialize, JsonSchema)]
pub struct DataAssignStaff {
    pub platform_admin_role: Option<PlatformAdminRole>,
    #[serde(default)]
    pub platform_permissions: Option<Vec<String>>,
}

#[derive(Serialize, JsonSchema)]
pub struct SystemFeaturesResponse {
    pub emergency_kill_switch: bool,
    pub global: HashMap<String, bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct DataSetFeature {
    pub feature: String,
    pub enabled: bool,
}

#[derive(Deserialize, JsonSchema)]
pub struct DataSetKillSwitch {
    pub enabled: bool,
}

#[derive(Deserialize, JsonSchema)]
pub struct DataSetUserFlags {
    pub flags: u32,
}

#[derive(Deserialize, JsonSchema)]
pub struct DataSetUserCosmetics {
    #[serde(default)]
    pub nameplate: Option<String>,
    #[serde(default)]
    pub role_colour: Option<String>,
    #[serde(default)]
    pub font: Option<String>,
    #[serde(default)]
    pub animation: Option<String>,
}

#[derive(Serialize, JsonSchema, Clone)]
pub struct UserRestrictionState {
    pub user_id: String,
    pub profile_locked: bool,
    pub username_frozen_until: Option<String>,
    pub display_name_frozen_until: Option<String>,
    pub media_quarantined: bool,
    pub profile_visibility_limited: bool,
}

#[derive(Deserialize, JsonSchema)]
pub struct DataSetUserRestrictions {
    #[serde(default)]
    pub profile_locked: Option<bool>,
    #[serde(default)]
    pub username_frozen_until: Option<String>,
    #[serde(default)]
    pub display_name_frozen_until: Option<String>,
    #[serde(default)]
    pub media_quarantined: Option<bool>,
    #[serde(default)]
    pub profile_visibility_limited: Option<bool>,
}

#[derive(Serialize, JsonSchema, Clone)]
pub struct EmailSystemResponse {
    pub smtp_enabled: bool,
    pub smtp_host: String,
    pub support_address: String,
    pub sender_name: String,
    pub reset_base_url: String,
    pub require_captcha: bool,
}

#[derive(Deserialize, JsonSchema)]
pub struct DataSetEmailSystem {
    #[serde(default)]
    pub support_address: Option<String>,
    #[serde(default)]
    pub sender_name: Option<String>,
    #[serde(default)]
    pub reset_base_url: Option<String>,
    #[serde(default)]
    pub require_captcha: Option<bool>,
}

#[derive(Serialize, JsonSchema)]
pub struct AuditLogResponse {
    pub entries: Vec<audit::AuditEntry>,
}

#[derive(Deserialize, rocket::form::FromForm, JsonSchema)]
pub struct AuditExportQuery {
    pub actor_id: Option<String>,
    pub action_prefix: Option<String>,
    pub target: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Serialize, JsonSchema)]
pub struct AuditExportResponse {
    pub entries: Vec<audit::AuditEntry>,
    pub chain_hash: String,
}

static EMAIL_SYSTEM_OVERRIDES: Lazy<Mutex<DataSetEmailSystem>> =
    Lazy::new(|| Mutex::new(DataSetEmailSystem {
        support_address: None,
        sender_name: None,
        reset_base_url: None,
        require_captcha: None,
    }));

const RESTRICTION_PROFILE_LOCKED: &str = "admin:restriction:profile_locked";
const RESTRICTION_USERNAME_FROZEN_UNTIL: &str = "admin:restriction:username_frozen_until";
const RESTRICTION_DISPLAY_NAME_FROZEN_UNTIL: &str = "admin:restriction:display_name_frozen_until";
const RESTRICTION_MEDIA_QUARANTINED: &str = "admin:restriction:media_quarantined";
const RESTRICTION_PROFILE_VISIBILITY_LIMITED: &str = "admin:restriction:profile_visibility_limited";

fn restriction_setting_keys() -> Vec<String> {
    vec![
        RESTRICTION_PROFILE_LOCKED.to_owned(),
        RESTRICTION_USERNAME_FROZEN_UNTIL.to_owned(),
        RESTRICTION_DISPLAY_NAME_FROZEN_UNTIL.to_owned(),
        RESTRICTION_MEDIA_QUARANTINED.to_owned(),
        RESTRICTION_PROFILE_VISIBILITY_LIMITED.to_owned(),
    ]
}

fn current_unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

fn parse_setting_bool(settings: &UserSettings, key: &str) -> bool {
    settings
        .get(key)
        .map(|(_, value)| value.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn parse_setting_optional(settings: &UserSettings, key: &str) -> Option<String> {
    settings.get(key).and_then(|(_, value)| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
}

fn decode_restrictions(user_id: String, settings: &UserSettings) -> UserRestrictionState {
    UserRestrictionState {
        user_id,
        profile_locked: parse_setting_bool(settings, RESTRICTION_PROFILE_LOCKED),
        username_frozen_until: parse_setting_optional(settings, RESTRICTION_USERNAME_FROZEN_UNTIL),
        display_name_frozen_until: parse_setting_optional(
            settings,
            RESTRICTION_DISPLAY_NAME_FROZEN_UNTIL,
        ),
        media_quarantined: parse_setting_bool(settings, RESTRICTION_MEDIA_QUARANTINED),
        profile_visibility_limited: parse_setting_bool(settings, RESTRICTION_PROFILE_VISIBILITY_LIMITED),
    }
}

fn encode_restrictions(timestamp: i64, state: &UserRestrictionState) -> UserSettings {
    HashMap::from([
        (
            RESTRICTION_PROFILE_LOCKED.to_owned(),
            (timestamp, state.profile_locked.to_string()),
        ),
        (
            RESTRICTION_USERNAME_FROZEN_UNTIL.to_owned(),
            (
                timestamp,
                state.username_frozen_until.clone().unwrap_or_default(),
            ),
        ),
        (
            RESTRICTION_DISPLAY_NAME_FROZEN_UNTIL.to_owned(),
            (
                timestamp,
                state.display_name_frozen_until.clone().unwrap_or_default(),
            ),
        ),
        (
            RESTRICTION_MEDIA_QUARANTINED.to_owned(),
            (timestamp, state.media_quarantined.to_string()),
        ),
        (
            RESTRICTION_PROFILE_VISIBILITY_LIMITED.to_owned(),
            (timestamp, state.profile_visibility_limited.to_string()),
        ),
    ])
}

async fn map_user_inspect(user: User) -> UserInspectResponse {
    let badges = user.get_badges().await;
    UserInspectResponse {
        badges,
        flags: user.flags.unwrap_or(0) as u32,
        platform_permissions: user.platform_permissions.unwrap_or_default(),
        id: user.id,
        username: user.username,
        discriminator: user.discriminator,
        display_name: user.display_name,
        privileged: user.privileged,
        platform_admin_role: user.platform_admin_role,
    }
}

fn user_matches_query(user: &User, query: Option<&str>) -> bool {
    let Some(raw) = query else {
        return true;
    };

    let needle = raw.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return true;
    }

    user.id.to_ascii_lowercase().contains(&needle)
        || user.username.to_ascii_lowercase().contains(&needle)
        || user
            .display_name
            .as_deref()
            .map(|value| value.to_ascii_lowercase().contains(&needle))
            .unwrap_or(false)
}

fn user_is_staff(user: &User) -> bool {
    const BADGE_COMMS_ADMIN: i32 = 1 << 11;
    const BADGE_COMMS_STAFF: i32 = 1 << 12;
    let badges = user.badges.unwrap_or_default();

    user.privileged
        || user.platform_admin_role.is_some()
        || (badges & BADGE_COMMS_ADMIN) != 0
        || (badges & BADGE_COMMS_STAFF) != 0
}

fn parse_severity(raw: Option<&str>) -> Result<Option<ReportSeverity>> {
    Ok(match raw {
        None => None,
        Some("Low") | Some("low") => Some(ReportSeverity::Low),
        Some("Medium") | Some("medium") => Some(ReportSeverity::Medium),
        Some("High") | Some("high") => Some(ReportSeverity::High),
        Some("Critical") | Some("critical") => Some(ReportSeverity::Critical),
        Some(other) => {
            return Err(create_error!(FailedValidation {
                error: format!("Invalid severity filter: {other}")
            }));
        }
    })
}

fn parse_priority(raw: Option<&str>) -> Result<Option<ReportPriority>> {
    Ok(match raw {
        None => None,
        Some("Low") | Some("low") => Some(ReportPriority::Low),
        Some("Normal") | Some("normal") => Some(ReportPriority::Normal),
        Some("High") | Some("high") => Some(ReportPriority::High),
        Some("Urgent") | Some("urgent") => Some(ReportPriority::Urgent),
        Some(other) => {
            return Err(create_error!(FailedValidation {
                error: format!("Invalid priority filter: {other}")
            }));
        }
    })
}

fn report_action_permission(action: &str) -> &'static str {
    match action {
        "ack" | "in_review" => admin_permissions::PERM_REPORTS_ASSIGN,
        "escalate_tier2" | "escalate" => admin_permissions::PERM_REPORTS_ESCALATE,
        "resolve" | "close_no_action" => admin_permissions::PERM_REPORTS_RESOLVE,
        "reject" => admin_permissions::PERM_REPORTS_REJECT,
        "merge_duplicate" | "reopen" | "request_more_info" => {
            admin_permissions::PERM_REPORTS_ACTIONS_WRITE
        }
        _ => admin_permissions::PERM_REPORTS_ACTIONS_WRITE,
    }
}

async fn email_system_snapshot() -> EmailSystemResponse {
    let cfg = config().await;
    let mut support_address = "support@localhost".to_owned();
    let mut sender_name = ".Comms Support".to_owned();
    let mut reset_base_url = "https://localhost/login".to_owned();
    let mut require_captcha = false;

    if let Ok(overrides) = EMAIL_SYSTEM_OVERRIDES.lock() {
        if let Some(value) = &overrides.support_address {
            support_address = value.clone();
        }
        if let Some(value) = &overrides.sender_name {
            sender_name = value.clone();
        }
        if let Some(value) = &overrides.reset_base_url {
            reset_base_url = value.clone();
        }
        if let Some(value) = overrides.require_captcha {
            require_captcha = value;
        }
    }

    EmailSystemResponse {
        smtp_enabled: !cfg.api.smtp.host.is_empty(),
        smtp_host: cfg.api.smtp.host.clone(),
        support_address,
        sender_name,
        reset_base_url,
        require_captcha,
    }
}

fn audit_chain_hash(entries: &[audit::AuditEntry]) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for entry in entries {
        entry.id.hash(&mut hasher);
        entry.timestamp.hash(&mut hasher);
        entry.actor_id.hash(&mut hasher);
        entry.action.hash(&mut hasher);
        entry.target.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

fn apply_report_status(
    report: &mut Report,
    status: ReportStatusString,
    rejection_reason: Option<String>,
) -> Result<()> {
    let now = Some(Timestamp::now_utc());
    report.last_transition_at = now;
    report.updated_at = now;
    report.status = match status {
        ReportStatusString::Created => ReportStatus::Created {},
        ReportStatusString::InReview => ReportStatus::InReview { assigned_at: now },
        ReportStatusString::Escalated => ReportStatus::Escalated { escalated_at: now },
        ReportStatusString::Resolved => {
            report.resolved_at = now;
            ReportStatus::Resolved { closed_at: now }
        }
        ReportStatusString::Rejected => {
            report.resolved_at = now;
            ReportStatus::Rejected {
                rejection_reason: rejection_reason
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| create_error!(FailedValidation {
                        error: "rejection_reason is required".to_owned()
                    }))?,
                closed_at: now,
            }
        }
    };

    Ok(())
}

/// # List Safety Reports
///
/// Fetch the global moderation queue for platform admins.
#[openapi(tag = "Admin")]
#[get("/reports?<query..>")]
pub async fn list_reports(
    db: &State<Database>,
    user: User,
    query: ReportQueueQuery,
) -> Result<Json<ReportQueueResponse>> {
    admin_permissions::ensure_permission(&user, admin_permissions::PERM_REPORTS_READ)?;
    let status = match query.status.as_deref() {
        None => None,
        Some("Created") | Some("created") => Some(ReportStatusString::Created),
        Some("InReview") | Some("in_review") | Some("inreview") | Some("in-review") => Some(ReportStatusString::InReview),
        Some("Escalated") | Some("escalated") => Some(ReportStatusString::Escalated),
        Some("Resolved") | Some("resolved") => Some(ReportStatusString::Resolved),
        Some("Rejected") | Some("rejected") => Some(ReportStatusString::Rejected),
        Some(other) => {
            return Err(create_error!(FailedValidation {
                error: format!("Invalid status filter: {other}")
            }));
        }
    };

    let severity = parse_severity(query.severity.as_deref())?;
    let priority = parse_priority(query.priority.as_deref())?;
    let mut reports = db.fetch_reports_by_status(status).await?;
    reports.retain(|report| {
        let severity_ok = severity
            .as_ref()
            .map(|value| &report.severity == value)
            .unwrap_or(true);
        let priority_ok = priority
            .as_ref()
            .map(|value| &report.priority == value)
            .unwrap_or(true);
        let assignee_ok = query
            .assignee_id
            .as_ref()
            .map(|value| report.assignee_id.as_deref() == Some(value.as_str()))
            .unwrap_or(true);
        let reviewer_ok = query
            .reviewer_id
            .as_ref()
            .map(|value| report.reviewer_id.as_deref() == Some(value.as_str()))
            .unwrap_or(true);
        severity_ok && priority_ok && assignee_ok && reviewer_ok
    });

    let priority_rank = |priority: &ReportPriority| -> usize {
        match priority {
            ReportPriority::Low => 0,
            ReportPriority::Normal => 1,
            ReportPriority::High => 2,
            ReportPriority::Urgent => 3,
        }
    };

    match query.sort.as_deref() {
        Some("risk_desc") => reports.sort_by(|a, b| b.risk_score.cmp(&a.risk_score)),
        Some("priority_desc") => {
            reports.sort_by(|a, b| priority_rank(&b.priority).cmp(&priority_rank(&a.priority)))
        }
        Some("oldest") => reports.sort_by(|a, b| a.id.cmp(&b.id)),
        _ => reports.sort_by(|a, b| b.id.cmp(&a.id)),
    }

    let total = reports.len();
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(150).clamp(1, 500);
    let reports = reports.into_iter().skip(offset).take(limit).collect::<Vec<_>>();
    Ok(Json(ReportQueueResponse {
        reports: reports.into_iter().map(Into::into).collect(),
        total,
    }))
}

/// # Get Safety Report
///
/// Fetch one moderation report by id.
#[openapi(tag = "Admin")]
#[get("/reports/<id>")]
pub async fn get_report(db: &State<Database>, user: User, id: String) -> Result<Json<revolt_models::v0::Report>> {
    admin_permissions::ensure_permission(&user, admin_permissions::PERM_REPORTS_READ)?;
    let report = db.fetch_report(&id).await?;
    Ok(Json(report.into()))
}

/// # Review Safety Report
///
/// Assign and transition a report through moderation lifecycle states.
#[openapi(tag = "Admin")]
#[patch("/reports/<id>", data = "<data>")]
pub async fn review_report(
    db: &State<Database>,
    user: User,
    id: String,
    data: Json<DataReviewReport>,
) -> Result<Json<revolt_models::v0::Report>> {
    admin_permissions::ensure_permission(&user, admin_permissions::PERM_REPORTS_REVIEW)?;
    let data = data.into_inner();

    let mut report = db.fetch_report(&id).await?;
    apply_report_status(&mut report, data.status, data.rejection_reason)?;
    report.reviewer_id = Some(user.id.clone());

    if let Some(notes) = data.notes {
        report.notes = notes;
    }

    if data.assignee_id.is_some() || matches!(report.status, ReportStatus::InReview { .. }) {
        report.assignee_id = data.assignee_id.or(Some(user.id.clone()));
    }

    audit::record(
        user.id.clone(),
        "report.review".to_owned(),
        id.clone(),
        HashMap::from([("status".to_owned(), format!("{:?}", report.status))]),
    );

    db.update_report(&id, &report).await?;
    Ok(Json(report.into()))
}

/// # Report Timeline
#[openapi(tag = "Admin")]
#[get("/reports/<id>/timeline")]
pub async fn report_timeline(
    _db: &State<Database>,
    user: User,
    id: String,
) -> Result<Json<ReportTimelineResponse>> {
    admin_permissions::ensure_permission(&user, admin_permissions::PERM_REPORTS_TIMELINE_READ)?;
    let entries = audit::list()
        .into_iter()
        .filter(|entry| entry.target == id)
        .collect::<Vec<_>>();

    Ok(Json(ReportTimelineResponse {
        report_id: id,
        entries,
    }))
}

/// # Report Snapshots
#[openapi(tag = "Admin")]
#[get("/reports/<id>/snapshots")]
pub async fn report_snapshots(
    db: &State<Database>,
    user: User,
    id: String,
) -> Result<Json<ReportSnapshotsResponse>> {
    admin_permissions::ensure_permission(&user, admin_permissions::PERM_REPORTS_READ)?;
    let snapshots = db
        .fetch_snapshots_by_report(&id)
        .await?
        .into_iter()
        .filter_map(|snapshot| serde_json::to_value(snapshot).ok())
        .collect::<Vec<_>>();
    Ok(Json(ReportSnapshotsResponse {
        report_id: id,
        snapshots,
    }))
}

/// # Report Workflow Action
#[openapi(tag = "Admin")]
#[post("/reports/<id>/actions", data = "<data>")]
pub async fn report_action(
    db: &State<Database>,
    user: User,
    id: String,
    data: Json<DataReportAction>,
) -> Result<Json<revolt_models::v0::Report>> {
    let data = data.into_inner();
    let permission = report_action_permission(&data.action);
    admin_permissions::ensure_permission(&user, permission)?;

    let mut report = db.fetch_report(&id).await?;
    let now = Some(Timestamp::now_utc());
    report.updated_at = now;
    report.last_transition_at = now;

    match data.action.as_str() {
        "ack" | "in_review" => {
            report.status = ReportStatus::InReview { assigned_at: now };
            report.assignee_id = data.assignee_id.clone().or(Some(user.id.clone()));
        }
        "escalate" | "escalate_tier2" => {
            report.status = ReportStatus::Escalated { escalated_at: now };
        }
        "resolve" => {
            report.status = ReportStatus::Resolved { closed_at: now };
            report.resolved_at = now;
        }
        "close_no_action" => {
            report.status = ReportStatus::Resolved { closed_at: now };
            report.resolved_at = now;
            if let Some(reason) = data.reason.as_ref() {
                report.notes = format!("No action closure reason: {reason}");
            }
        }
        "reject" => {
            let reason = data
                .reason
                .clone()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| create_error!(FailedValidation {
                    error: "reason is required for reject".to_owned()
                }))?;
            report.status = ReportStatus::Rejected {
                rejection_reason: reason,
                closed_at: now,
            };
            report.resolved_at = now;
        }
        "reopen" => {
            report.status = ReportStatus::Created {};
            report.resolved_at = None;
        }
        "merge_duplicate" => {
            let merge_into = data
                .merge_into
                .clone()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| create_error!(FailedValidation {
                    error: "merge_into is required for merge_duplicate".to_owned()
                }))?;
            if merge_into != id && !report.related_report_ids.contains(&merge_into) {
                report.related_report_ids.push(merge_into);
            }
        }
        "request_more_info" => {
            if let Some(reason) = data.reason.as_ref() {
                report.notes = format!("Requested additional info: {reason}");
            }
        }
        other => {
            return Err(create_error!(FailedValidation {
                error: format!("Unknown report action: {other}")
            }));
        }
    }

    report.reviewer_id = Some(user.id.clone());
    db.update_report(&id, &report).await?;
    audit::record(
        user.id,
        "report.action".to_owned(),
        id,
        HashMap::from([
            ("action".to_owned(), data.action),
            ("reason".to_owned(), data.reason.unwrap_or_else(|| "none".to_owned())),
        ]),
    );
    Ok(Json(report.into()))
}

/// # Safety Dashboard
///
/// Aggregate queue-level counts for supervisory dashboards.
#[openapi(tag = "Admin")]
#[get("/reports/dashboard")]
pub async fn reports_dashboard(
    db: &State<Database>,
    user: User,
) -> Result<Json<ReportDashboardResponse>> {
    admin_permissions::ensure_permission(&user, admin_permissions::PERM_REPORTS_READ)?;
    let reports = db.fetch_reports_by_status(None).await?;

    let mut created = 0usize;
    let mut in_review = 0usize;
    let mut escalated = 0usize;
    let mut resolved = 0usize;
    let mut rejected = 0usize;

    for report in &reports {
        match report.status {
            ReportStatus::Created {} => created += 1,
            ReportStatus::InReview { .. } => in_review += 1,
            ReportStatus::Escalated { .. } => escalated += 1,
            ReportStatus::Resolved { .. } => resolved += 1,
            ReportStatus::Rejected { .. } => rejected += 1,
        }
    }

    Ok(Json(ReportDashboardResponse {
        total: reports.len(),
        created,
        in_review,
        escalated,
        resolved,
        rejected,
    }))
}

/// # Inspect User
///
/// Fetch account-level metadata for control-plane investigations.
#[openapi(tag = "Admin")]
#[get("/users?<query..>")]
pub async fn list_users(
    db: &State<Database>,
    user: User,
    query: UsersListQuery,
) -> Result<Json<UsersListResponse>> {
    admin_permissions::ensure_permission(&user, admin_permissions::PERM_USERS_READ)?;

    let limit = query.limit.unwrap_or(150).clamp(1, 500);
    let users = db.fetch_recent_users(limit.saturating_mul(3).min(500)).await?;

    let mut mapped = Vec::with_capacity(limit);
    for u in users {
        if user_matches_query(&u, query.q.as_deref()) {
            mapped.push(map_user_inspect(u).await);
            if mapped.len() >= limit {
                break;
            }
        }
    }

    Ok(Json(UsersListResponse { users: mapped }))
}

/// # Inspect User
///
/// Fetch account-level metadata for control-plane investigations.
#[openapi(tag = "Admin")]
#[get("/users/<id>")]
pub async fn inspect_user(
    db: &State<Database>,
    user: User,
    id: String,
) -> Result<Json<UserInspectResponse>> {
    admin_permissions::ensure_permission(&user, admin_permissions::PERM_USERS_READ)?;
    let target = db.fetch_user(&id).await?;
    Ok(Json(map_user_inspect(target).await))
}

/// # Set User Account Flags
///
/// Update user moderation flags (suspended / deleted / banned bitfield).
#[openapi(tag = "Admin")]
#[patch("/users/<id>/flags", data = "<data>")]
pub async fn set_user_flags(
    db: &State<Database>,
    user: User,
    id: String,
    data: Json<DataSetUserFlags>,
) -> Result<Json<UserInspectResponse>> {
    admin_permissions::ensure_permission(&user, admin_permissions::PERM_USERS_FLAGS_WRITE)?;
    let data = data.into_inner();

    let mut target = db.fetch_user(&id).await?;
    let partial = PartialUser {
        flags: Some(data.flags as i32),
        ..Default::default()
    };
    target.update(db, partial, vec![]).await?;
    audit::record(
        user.id,
        "user.flags.set".to_owned(),
        id,
        HashMap::from([("flags".to_owned(), data.flags.to_string())]),
    );
    Ok(Json(map_user_inspect(target).await))
}

/// # Set User Cosmetics
#[openapi(tag = "Admin")]
#[patch("/users/<id>/cosmetics", data = "<data>")]
pub async fn set_user_cosmetics(
    db: &State<Database>,
    user: User,
    id: String,
    data: Json<DataSetUserCosmetics>,
) -> Result<Json<UserInspectResponse>> {
    admin_permissions::ensure_permission(&user, admin_permissions::PERM_USERS_COSMETICS_WRITE)?;
    let data = data.into_inner();
    let mut target = db.fetch_user(&id).await?;

    let mut profile = target.profile.clone().unwrap_or_default();
    let mut cosmetics = profile.cosmetics.clone().unwrap_or_default();
    if data.nameplate.is_some() {
        cosmetics.nameplate = data.nameplate;
    }
    if data.role_colour.is_some() {
        cosmetics.role_colour = data.role_colour;
    }
    if data.font.is_some() {
        cosmetics.font = data.font;
    }
    if data.animation.is_some() {
        cosmetics.animation = data.animation;
    }
    profile.cosmetics = Some(cosmetics.clone());

    target
        .update(
            db,
            PartialUser {
                profile: Some(profile),
                ..Default::default()
            },
            vec![],
        )
        .await?;

    audit::record(
        user.id,
        "user.cosmetics.set".to_owned(),
        id,
        HashMap::from([
            (
                "nameplate".to_owned(),
                cosmetics
                    .nameplate
                    .clone()
                    .unwrap_or_else(|| "none".to_owned()),
            ),
            (
                "role_colour".to_owned(),
                cosmetics
                    .role_colour
                    .clone()
                    .unwrap_or_else(|| "none".to_owned()),
            ),
        ]),
    );

    Ok(Json(map_user_inspect(target).await))
}

/// # Set User Restrictions
#[openapi(tag = "Admin")]
#[patch("/users/<id>/restrictions", data = "<data>")]
pub async fn set_user_restrictions(
    db: &State<Database>,
    user: User,
    id: String,
    data: Json<DataSetUserRestrictions>,
) -> Result<Json<UserRestrictionState>> {
    admin_permissions::ensure_permission(
        &user,
        admin_permissions::PERM_USERS_RESTRICTIONS_WRITE,
    )?;
    let data = data.into_inner();

    let mut current = decode_restrictions(
        id.clone(),
        &db.fetch_user_settings(&id, &restriction_setting_keys())
            .await
            .unwrap_or_default(),
    );

    if let Some(value) = data.profile_locked {
        current.profile_locked = value;
    }
    if data.username_frozen_until.is_some() {
        current.username_frozen_until = data.username_frozen_until;
    }
    if data.display_name_frozen_until.is_some() {
        current.display_name_frozen_until = data.display_name_frozen_until;
    }
    if let Some(value) = data.media_quarantined {
        current.media_quarantined = value;
    }
    if let Some(value) = data.profile_visibility_limited {
        current.profile_visibility_limited = value;
    }

    db.set_user_settings(&id, &encode_restrictions(current_unix_millis(), &current))
        .await?;

    audit::record(
        user.id,
        "user.restrictions.set".to_owned(),
        id,
        HashMap::from([
            ("profile_locked".to_owned(), current.profile_locked.to_string()),
            (
                "media_quarantined".to_owned(),
                current.media_quarantined.to_string(),
            ),
            (
                "profile_visibility_limited".to_owned(),
                current.profile_visibility_limited.to_string(),
            ),
        ]),
    );
    Ok(Json(current))
}

/// # Get User Restrictions
#[openapi(tag = "Admin")]
#[get("/users/<id>/restrictions")]
pub async fn get_user_restrictions(
    db: &State<Database>,
    user: User,
    id: String,
) -> Result<Json<UserRestrictionState>> {
    admin_permissions::ensure_permission(&user, admin_permissions::PERM_USERS_READ)?;
    let settings = db
        .fetch_user_settings(&id, &restriction_setting_keys())
        .await
        .unwrap_or_default();
    Ok(Json(decode_restrictions(id, &settings)))
}

/// # List Staff
///
/// Fetch one or more staff records by id list.
#[openapi(tag = "Admin")]
#[get("/staff?<query..>")]
pub async fn list_staff(
    db: &State<Database>,
    user: User,
    query: StaffListQuery,
) -> Result<Json<StaffListResponse>> {
    admin_permissions::ensure_permission(&user, admin_permissions::PERM_STAFF_READ)?;

    let ids: Vec<String> = query
        .ids
        .clone()
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect();

    let limit = query.limit.unwrap_or(150).clamp(1, 500);
    let users = if ids.is_empty() {
        db.fetch_recent_users(limit.saturating_mul(4).min(500)).await?
    } else {
        db.fetch_users(&ids).await?
    };

    let mut mapped = Vec::with_capacity(limit);
    for u in users {
        if !user_is_staff(&u) || !user_matches_query(&u, query.q.as_deref()) {
            continue;
        }

        mapped.push(map_user_inspect(u).await);
        if mapped.len() >= limit {
            break;
        }
    }
    Ok(Json(StaffListResponse { users: mapped }))
}

/// # Staff Permission Catalog
#[openapi(tag = "Admin")]
#[get("/staff/permissions/catalog")]
pub async fn staff_permissions_catalog(
    _db: &State<Database>,
    user: User,
) -> Result<Json<StaffPermissionsCatalogResponse>> {
    admin_permissions::ensure_permission(&user, admin_permissions::PERM_STAFF_READ)?;

    let templates = admin_permissions::role_templates()
        .into_iter()
        .map(|(role, perms)| (role, perms))
        .collect::<HashMap<_, _>>();

    Ok(Json(StaffPermissionsCatalogResponse {
        catalog: admin_permissions::PERMISSION_CATALOG
            .iter()
            .map(|value| value.to_string())
            .collect(),
        role_templates: templates,
    }))
}

/// # Assign Staff Role
///
/// Assign or update control-plane role and scoped feature grants.
#[openapi(tag = "Admin")]
#[patch("/staff/<id>", data = "<data>")]
pub async fn assign_staff(
    db: &State<Database>,
    user: User,
    id: String,
    data: Json<DataAssignStaff>,
) -> Result<Json<UserInspectResponse>> {
    let data = data.into_inner();
    admin_permissions::ensure_owner_delegation(
        &user,
        data.platform_permissions.as_deref().unwrap_or_default(),
    )?;
    let mut target = db.fetch_user(&id).await?;
    if matches!(data.platform_admin_role, Some(PlatformAdminRole::PlatformOwner))
        && !matches!(user.platform_admin_role, Some(PlatformAdminRole::PlatformOwner))
        && !user.privileged
    {
        return Err(create_error!(NotPrivileged));
    }

    if let Some(ref permissions) = data.platform_permissions {
        admin_permissions::validate_permissions(permissions)?;
    }

    let partial = PartialUser {
        platform_admin_role: data.platform_admin_role,
        platform_permissions: data.platform_permissions,
        ..Default::default()
    };
    target.update(db, partial, vec![]).await?;
    audit::record(
        user.id.clone(),
        "staff.assign".to_owned(),
        id,
        HashMap::from([
            (
                "role".to_owned(),
                target
                    .platform_admin_role
                    .as_ref()
                    .map(|role| format!("{role:?}"))
                    .unwrap_or_else(|| "None".to_owned()),
            ),
            (
                "permissions".to_owned(),
                target
                    .platform_permissions
                    .as_ref()
                    .map(|values| values.join(","))
                    .unwrap_or_else(|| "None".to_owned()),
            ),
        ]),
    );
    Ok(Json(map_user_inspect(target).await))
}

/// # Get System Feature Toggles
#[openapi(tag = "Admin")]
#[get("/system/features")]
pub async fn get_system_features(
    _db: &State<Database>,
    user: User,
) -> Result<Json<SystemFeaturesResponse>> {
    admin_permissions::ensure_permission(&user, admin_permissions::PERM_POLICY_READ)?;
    Ok(Json(SystemFeaturesResponse {
        emergency_kill_switch: rollout::emergency_kill_switch(),
        global: rollout::global_features(),
    }))
}

/// # Set System Feature Toggle
#[openapi(tag = "Admin")]
#[patch("/system/features", data = "<data>")]
pub async fn set_system_feature(
    _db: &State<Database>,
    user: User,
    data: Json<DataSetFeature>,
) -> Result<Json<SystemFeaturesResponse>> {
    admin_permissions::ensure_permission(&user, admin_permissions::PERM_SYSTEM_FEATURES_WRITE)?;
    let data = data.into_inner();
    let feature = data.feature.clone();
    rollout::set_global_feature(data.feature, data.enabled);
    audit::record(
        user.id,
        "system.feature.set".to_owned(),
        feature,
        HashMap::from([("enabled".to_owned(), data.enabled.to_string())]),
    );
    Ok(Json(SystemFeaturesResponse {
        emergency_kill_switch: rollout::emergency_kill_switch(),
        global: rollout::global_features(),
    }))
}

/// # Set Emergency Kill Switch
#[openapi(tag = "Admin")]
#[patch("/system/kill-switch", data = "<data>")]
pub async fn set_system_kill_switch(
    _db: &State<Database>,
    user: User,
    data: Json<DataSetKillSwitch>,
) -> Result<Json<SystemFeaturesResponse>> {
    admin_permissions::ensure_permission(
        &user,
        admin_permissions::PERM_SYSTEM_KILL_SWITCH_WRITE,
    )?;
    rollout::set_emergency_kill_switch(data.enabled);
    audit::record(
        user.id,
        "system.kill_switch".to_owned(),
        "global".to_owned(),
        HashMap::from([("enabled".to_owned(), data.enabled.to_string())]),
    );
    Ok(Json(SystemFeaturesResponse {
        emergency_kill_switch: rollout::emergency_kill_switch(),
        global: rollout::global_features(),
    }))
}

/// # Get Email System Configuration
#[openapi(tag = "Admin")]
#[get("/system/email")]
pub async fn get_system_email(
    _db: &State<Database>,
    user: User,
) -> Result<Json<EmailSystemResponse>> {
    admin_permissions::ensure_permission(&user, admin_permissions::PERM_POLICY_READ)?;
    Ok(Json(email_system_snapshot().await))
}

/// # Update Email System Configuration
///
/// Runtime-only overrides for support sender metadata exposed to staff tooling.
#[openapi(tag = "Admin")]
#[patch("/system/email", data = "<data>")]
pub async fn set_system_email(
    _db: &State<Database>,
    user: User,
    data: Json<DataSetEmailSystem>,
) -> Result<Json<EmailSystemResponse>> {
    admin_permissions::ensure_permission(&user, admin_permissions::PERM_SYSTEM_EMAIL_WRITE)?;
    let data = data.into_inner();
    if let Ok(mut overrides) = EMAIL_SYSTEM_OVERRIDES.lock() {
        if data.support_address.is_some() {
            overrides.support_address = data.support_address.clone();
        }
        if data.sender_name.is_some() {
            overrides.sender_name = data.sender_name.clone();
        }
        if data.reset_base_url.is_some() {
            overrides.reset_base_url = data.reset_base_url.clone();
        }
        if data.require_captcha.is_some() {
            overrides.require_captcha = data.require_captcha;
        }
    }
    audit::record(
        user.id,
        "system.email.set".to_owned(),
        "global".to_owned(),
        HashMap::from([
            (
                "support_address".to_owned(),
                data.support_address.unwrap_or_else(|| "(unchanged)".to_owned()),
            ),
            (
                "sender_name".to_owned(),
                data.sender_name.unwrap_or_else(|| "(unchanged)".to_owned()),
            ),
        ]),
    );
    Ok(Json(email_system_snapshot().await))
}

/// # Get Admin Audit Log
#[openapi(tag = "Admin")]
#[get("/audit")]
pub async fn get_audit_log(_db: &State<Database>, user: User) -> Result<Json<AuditLogResponse>> {
    admin_permissions::ensure_permission(&user, admin_permissions::PERM_AUDIT_READ)?;
    Ok(Json(AuditLogResponse {
        entries: audit::list(),
    }))
}

/// # Export Admin Audit Log
#[openapi(tag = "Admin")]
#[get("/audit/export?<query..>")]
pub async fn export_audit_log(
    _db: &State<Database>,
    user: User,
    query: AuditExportQuery,
) -> Result<Json<AuditExportResponse>> {
    admin_permissions::ensure_permission(&user, admin_permissions::PERM_AUDIT_READ)?;
    let limit = query.limit.unwrap_or(500).clamp(1, 5000);
    let entries = audit::list()
        .into_iter()
        .filter(|entry| {
            let actor_ok = query
                .actor_id
                .as_ref()
                .map(|value| &entry.actor_id == value)
                .unwrap_or(true);
            let action_ok = query
                .action_prefix
                .as_ref()
                .map(|value| entry.action.starts_with(value))
                .unwrap_or(true);
            let target_ok = query
                .target
                .as_ref()
                .map(|value| &entry.target == value)
                .unwrap_or(true);
            actor_ok && action_ok && target_ok
        })
        .take(limit)
        .collect::<Vec<_>>();

    Ok(Json(AuditExportResponse {
        chain_hash: audit_chain_hash(&entries),
        entries,
    }))
}

/// # Sensitive Admin Action Feed
#[openapi(tag = "Admin")]
#[get("/audit/sensitive")]
pub async fn sensitive_audit_feed(
    _db: &State<Database>,
    user: User,
) -> Result<Json<AuditLogResponse>> {
    admin_permissions::ensure_permission(&user, admin_permissions::PERM_AUDIT_READ)?;
    let entries = audit::list()
        .into_iter()
        .filter(|entry| {
            entry.action.starts_with("staff.")
                || entry.action.starts_with("system.")
                || entry.action.starts_with("user.restrictions")
                || entry.action.starts_with("user.cosmetics")
                || entry.action.starts_with("report.action")
        })
        .collect::<Vec<_>>();

    Ok(Json(AuditLogResponse { entries }))
}
