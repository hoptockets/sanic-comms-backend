use revolt_result::Result;
use revolt_models::v0::ReportStatusString;

use crate::ReferenceDb;
use crate::Report;

use super::AbstractReport;

#[async_trait]
impl AbstractReport for ReferenceDb {
    /// Insert a new report into the database
    async fn insert_report(&self, report: &Report) -> Result<()> {
        let mut reports = self.safety_reports.lock().await;
        if reports.contains_key(&report.id) {
            Err(create_database_error!("insert", "report"))
        } else {
            reports.insert(report.id.to_string(), report.clone());
            Ok(())
        }
    }

    async fn fetch_report(&self, id: &str) -> Result<Report> {
        self.safety_reports
            .lock()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| create_error!(NotFound))
    }

    async fn fetch_reports_by_status(&self, status: Option<ReportStatusString>) -> Result<Vec<Report>> {
        let reports = self.safety_reports.lock().await;
        let matches_status = |report: &Report| -> bool {
            match (&status, &report.status) {
                (None, _) => true,
                (Some(ReportStatusString::Created), revolt_models::v0::ReportStatus::Created {}) => true,
                (Some(ReportStatusString::InReview), revolt_models::v0::ReportStatus::InReview { .. }) => true,
                (Some(ReportStatusString::Escalated), revolt_models::v0::ReportStatus::Escalated { .. }) => true,
                (Some(ReportStatusString::Rejected), revolt_models::v0::ReportStatus::Rejected { .. }) => true,
                (Some(ReportStatusString::Resolved), revolt_models::v0::ReportStatus::Resolved { .. }) => true,
                _ => false,
            }
        };

        Ok(reports
            .values()
            .filter(|report| matches_status(report))
            .cloned()
            .collect())
    }

    async fn update_report(&self, id: &str, report: &Report) -> Result<()> {
        let mut reports = self.safety_reports.lock().await;
        if reports.contains_key(id) {
            reports.insert(id.to_string(), report.clone());
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }
}
