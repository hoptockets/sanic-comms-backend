use revolt_models::v0::{
    ReportContext, ReportPriority, ReportSeverity, ReportStatus, ReportedContent,
    ReporterMetadata,
};

auto_derived!(
    /// User-generated platform moderation report
    pub struct Report {
        /// Unique Id
        #[serde(rename = "_id")]
        pub id: String,
        /// Id of the user creating this report
        pub author_id: String,
        /// Reported content
        pub content: ReportedContent,
        /// Additional report context
        pub additional_context: String,
        /// Structured report context derived from content type.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub report_context: Option<ReportContext>,
        /// Optional source metadata provided by reporter client.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub reporter_metadata: Option<ReporterMetadata>,
        /// Severity used by triage.
        #[serde(default = "default_report_severity")]
        pub severity: ReportSeverity,
        /// Priority used by queue ordering.
        #[serde(default = "default_report_priority")]
        pub priority: ReportPriority,
        /// Optional risk score in range 0..=100.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub risk_score: Option<u8>,
        /// Optional confidence score in range 0..=100.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub confidence_score: Option<u8>,
        /// Optional dedupe key for report collapsing.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub dedupe_key: Option<String>,
        /// Additional reports merged with this one.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub related_report_ids: Vec<String>,
        /// Time report was created.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub created_at: Option<iso8601_timestamp::Timestamp>,
        /// Last mutable update timestamp.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub updated_at: Option<iso8601_timestamp::Timestamp>,
        /// Last status transition timestamp.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub last_transition_at: Option<iso8601_timestamp::Timestamp>,
        /// Time report reached a terminal state.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub resolved_at: Option<iso8601_timestamp::Timestamp>,
        /// SLA deadline for moderation action.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub sla_deadline: Option<iso8601_timestamp::Timestamp>,
        /// Whether report has breached its SLA.
        #[serde(default)]
        pub breach_state: bool,
        /// Status of the report
        #[serde(flatten)]
        pub status: ReportStatus,
        /// Additional notes included on the report
        #[serde(default)]
        pub notes: String,
        /// Assigned platform moderator id
        #[serde(skip_serializing_if = "Option::is_none")]
        pub assignee_id: Option<String>,
        /// Last reviewer that changed report status
        #[serde(skip_serializing_if = "Option::is_none")]
        pub reviewer_id: Option<String>,
    }
);

fn default_report_severity() -> ReportSeverity {
    ReportSeverity::Medium
}

fn default_report_priority() -> ReportPriority {
    ReportPriority::Normal
}
