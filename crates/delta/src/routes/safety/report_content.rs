use iso8601_timestamp::Timestamp;
use revolt_database::{events::client::EventV1, Database, Report, Snapshot, SnapshotContent, User};
use revolt_models::v0::{
    ReportContext, ReportPriority, ReportSeverity, ReportStatus, ReportedContent,
    ReporterMetadata,
};
use revolt_result::{create_error, Result};
use rocket_empty::EmptyResponse;
use serde::Deserialize;
use ulid::Ulid;
use validator::Validate;

use rocket::{serde::json::Json, State};

/// # Report Data
#[derive(Validate, Deserialize, JsonSchema)]
pub struct DataReportContent {
    /// Content being reported
    content: ReportedContent,
    /// Additional report description
    #[validate(length(min = 0, max = 1000))]
    #[serde(default)]
    additional_context: String,
    /// Optional severity hint from reporting client.
    #[serde(default)]
    severity: Option<ReportSeverity>,
    /// Optional priority hint from reporting client.
    #[serde(default)]
    priority: Option<ReportPriority>,
    /// Optional risk score (0-100) from heuristic clients.
    #[serde(default)]
    risk_score: Option<u8>,
    /// Optional confidence score (0-100) from heuristic clients.
    #[serde(default)]
    confidence_score: Option<u8>,
    /// Optional client metadata.
    #[serde(default)]
    reporter_metadata: Option<ReporterMetadata>,
}

/// # Report Content
///
/// Report a piece of content to the moderation team.
#[openapi(tag = "User Safety")]
#[post("/report", data = "<data>")]
pub async fn report_content(
    db: &State<Database>,
    user: User,
    data: Json<DataReportContent>,
) -> Result<EmptyResponse> {
    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    // Bots cannot create reports
    if user.bot.is_some() {
        return Err(create_error!(IsBot));
    }

    // Find the content and create a snapshot of it
    // Also retrieve any references to Files
    let (snapshots, files, report_context): (Vec<SnapshotContent>, Vec<String>, Option<ReportContext>) =
        match &data.content {
        ReportedContent::Message { id, .. } => {
            let message = db.fetch_message(id).await?;

            // Users cannot report themselves
            if message.author == user.id {
                return Err(create_error!(CannotReportYourself));
            }

            let report_context = ReportContext::Message {
                message_id: id.clone(),
                channel_id: Some(message.channel.clone()),
                server_id: None,
                attachment_count: message
                    .attachments
                    .as_ref()
                    .map(|attachments| attachments.len())
                    .unwrap_or(0),
            };

            let (snapshot, files) = SnapshotContent::generate_from_message(db, message).await?;
            (vec![snapshot], files, Some(report_context))
        }
        ReportedContent::Server { id, .. } => {
            let server = db.fetch_server(id).await?;
            let report_context = ReportContext::Server {
                server_id: id.clone(),
                owner_id: Some(server.owner.clone()),
            };

            let (snapshot, files) = SnapshotContent::generate_from_server(server)?;
            (vec![snapshot], files, Some(report_context))
        }
        ReportedContent::User { id, message_id, .. } => {
            let reported_user = db.fetch_user(id).await?;

            // Users cannot report themselves
            if reported_user.id == user.id {
                return Err(create_error!(CannotReportYourself));
            }

            // Determine if there is a message provided as context
            let message = if let Some(id) = message_id {
                db.fetch_message(id).await.ok()
            } else {
                None
            };

            let report_context = ReportContext::User {
                user_id: id.clone(),
                linked_message_id: message_id.clone(),
            };

            let (snapshot, files) = SnapshotContent::generate_from_user(reported_user)?;

            if let Some(message) = message {
                let (message_snapshot, message_files) =
                    SnapshotContent::generate_from_message(db, message).await?;
                (
                    vec![snapshot, message_snapshot],
                    [files, message_files].concat(),
                    Some(report_context),
                )
            } else {
                (vec![snapshot], files, Some(report_context))
            }
        }
    };

    // Mark all the attachments as reported
    for file in files {
        db.mark_attachment_as_reported(&file).await?;
    }

    // Generate an id for the report
    let id = Ulid::new().to_string();
    let dedupe_key = format!(
        "{}::{}::{}",
        user.id,
        match &data.content {
            ReportedContent::Message { id, .. } => format!("Message:{id}"),
            ReportedContent::Server { id, .. } => format!("Server:{id}"),
            ReportedContent::User { id, .. } => format!("User:{id}"),
        },
        data.additional_context.trim().to_ascii_lowercase()
    );

    // Collapse duplicate open reports in a short time window.
    let mut existing = db.fetch_reports_by_status(None).await?;
    if let Some(existing_report) = existing.iter_mut().find(|report| {
        let same_author = report.author_id == user.id;
        let same_dedupe = report.dedupe_key.as_deref() == Some(dedupe_key.as_str());
        let open = matches!(
            report.status,
            ReportStatus::Created {} | ReportStatus::InReview { .. } | ReportStatus::Escalated { .. }
        );
        same_author && same_dedupe && open
    }) {
        if !existing_report.related_report_ids.contains(&id) {
            existing_report.related_report_ids.push(id);
        }
        existing_report.updated_at = Some(Timestamp::now_utc());
        db.update_report(&existing_report.id, existing_report).await?;
        return Ok(EmptyResponse);
    }

    // Insert all new generated snapshots
    for content in snapshots {
        // Save a snapshot of the content
        let snapshot = Snapshot {
            id: Ulid::new().to_string(),
            report_id: id.to_string(),
            content,
        };

        db.insert_snapshot(&snapshot).await?;
    }

    // Save the report
    let report = Report {
        id,
        author_id: user.id,
        content: data.content,
        additional_context: data.additional_context,
        report_context,
        reporter_metadata: data.reporter_metadata,
        severity: data.severity.unwrap_or(ReportSeverity::Medium),
        priority: data.priority.unwrap_or(ReportPriority::Normal),
        risk_score: data.risk_score.filter(|value| *value <= 100),
        confidence_score: data.confidence_score.filter(|value| *value <= 100),
        dedupe_key: Some(dedupe_key),
        related_report_ids: Vec::new(),
        created_at: Some(Timestamp::now_utc()),
        updated_at: None,
        last_transition_at: Some(Timestamp::now_utc()),
        resolved_at: None,
        sla_deadline: None,
        breach_state: false,
        status: ReportStatus::Created {},
        notes: String::new(),
        assignee_id: None,
        reviewer_id: None,
    };

    db.insert_report(&report).await?;

    EventV1::ReportCreate(report.into()).global().await;

    Ok(EmptyResponse)
}
