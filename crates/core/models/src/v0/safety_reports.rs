use iso8601_timestamp::Timestamp;

auto_derived!(
    /// Severity assigned to moderation report
    pub enum ReportSeverity {
        Low,
        Medium,
        High,
        Critical,
    }

    /// Priority used by moderation queue ordering
    pub enum ReportPriority {
        Low,
        Normal,
        High,
        Urgent,
    }

    /// Additional reporter-side metadata captured on submission.
    pub struct ReporterMetadata {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub client_platform: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub locale: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub app_version: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub network_fingerprint_hash: Option<String>,
    }

    /// Structured report context expanded by report type.
    #[serde(tag = "type")]
    pub enum ReportContext {
        Message {
            message_id: String,
            #[serde(default)]
            channel_id: Option<String>,
            #[serde(default)]
            server_id: Option<String>,
            #[serde(default)]
            attachment_count: usize,
        },
        Server {
            server_id: String,
            #[serde(default)]
            owner_id: Option<String>,
        },
        User {
            user_id: String,
            #[serde(default)]
            linked_message_id: Option<String>,
        },
    }

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
        pub created_at: Option<Timestamp>,
        /// Last mutable update timestamp.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub updated_at: Option<Timestamp>,
        /// Last status transition timestamp.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub last_transition_at: Option<Timestamp>,
        /// Time report reached a terminal state.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub resolved_at: Option<Timestamp>,
        /// SLA deadline for moderation action.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub sla_deadline: Option<Timestamp>,
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

    /// Reason for reporting content (message or server)
    pub enum ContentReportReason {
        /// No reason has been specified
        NoneSpecified,

        /// Illegal content catch-all reason
        Illegal,

        /// Selling or facilitating use of drugs or other illegal goods
        IllegalGoods,

        /// Extortion or blackmail
        IllegalExtortion,

        /// Revenge or child pornography
        IllegalPornography,

        /// Illegal hacking activity
        IllegalHacking,

        /// Extreme violence, gore, or animal cruelty
        /// With exception to violence potrayed in media / creative arts
        ExtremeViolence,

        /// Content that promotes harm to others / self
        PromotesHarm,

        /// Unsolicited advertisements
        UnsolicitedSpam,

        /// This is a raid
        Raid,

        /// Spam or platform abuse
        SpamAbuse,

        /// Scams or fraud
        ScamsFraud,

        /// Distribution of malware or malicious links
        Malware,

        /// Harassment or abuse targeted at another user
        Harassment,
    }

    /// Reason for reporting a user
    pub enum UserReportReason {
        /// No reason has been specified
        NoneSpecified,

        /// Unsolicited advertisements
        UnsolicitedSpam,

        /// User is sending spam or otherwise abusing the platform
        SpamAbuse,

        /// User's profile contains inappropriate content for a general audience
        InappropriateProfile,

        /// User is impersonating another user
        Impersonation,

        /// User is evading a ban
        BanEvasion,

        /// User is not of minimum age to use the platform
        Underage,
    }

    /// The content being reported
    #[serde(tag = "type")]
    pub enum ReportedContent {
        /// Report a message
        Message {
            /// ID of the message
            id: String,
            /// Reason for reporting message
            report_reason: ContentReportReason,
        },
        /// Report a server
        Server {
            /// ID of the server
            id: String,
            /// Reason for reporting server
            report_reason: ContentReportReason,
        },
        /// Report a user
        User {
            /// ID of the user
            id: String,
            /// Reason for reporting a user
            report_reason: UserReportReason,
            /// Message context
            message_id: Option<String>,
        },
    }

    /// Status of the report
    #[serde(tag = "status")]
    pub enum ReportStatus {
        /// Report is waiting for triage / action
        Created {},
        /// Report has been assigned and is being reviewed
        InReview { assigned_at: Option<Timestamp> },
        /// Report has been escalated to supervisors
        Escalated { escalated_at: Option<Timestamp> },

        /// Report was rejected
        Rejected {
            rejection_reason: String,
            closed_at: Option<Timestamp>,
        },

        /// Report was actioned and resolved
        Resolved { closed_at: Option<Timestamp> },
    }

    /// Just the status of the report
    pub enum ReportStatusString {
        /// Report is waiting for triage / action
        Created,
        /// Report is being investigated by a moderator
        InReview,
        /// Report is escalated to supervisors
        Escalated,

        /// Report was rejected
        Rejected,

        /// Report was actioned and resolved
        Resolved,
    }
);

fn default_report_severity() -> ReportSeverity {
    ReportSeverity::Medium
}

fn default_report_priority() -> ReportPriority {
    ReportPriority::Normal
}
