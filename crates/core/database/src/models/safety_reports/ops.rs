use revolt_result::Result;
use revolt_models::v0::ReportStatusString;

use crate::Report;

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractReport: Sync + Send {
    /// Insert a new report into the database
    async fn insert_report(&self, report: &Report) -> Result<()>;
    /// Fetch a single report by id
    async fn fetch_report(&self, id: &str) -> Result<Report>;
    /// Fetch reports with optional status filter
    async fn fetch_reports_by_status(&self, status: Option<ReportStatusString>) -> Result<Vec<Report>>;
    /// Persist report changes
    async fn update_report(&self, id: &str, report: &Report) -> Result<()>;
}
