use bson::doc;
use futures::StreamExt;
use revolt_models::v0::ReportStatusString;
use revolt_result::Result;

use crate::MongoDb;
use crate::Report;

use super::AbstractReport;

static COL: &str = "safety_reports";

#[async_trait]
impl AbstractReport for MongoDb {
    /// Insert a new report into the database
    async fn insert_report(&self, report: &Report) -> Result<()> {
        query!(self, insert_one, COL, &report).map(|_| ())
    }

    async fn fetch_report(&self, id: &str) -> Result<Report> {
        query!(self, find_one_by_id, COL, id)?.ok_or_else(|| create_error!(NotFound))
    }

    async fn fetch_reports_by_status(&self, status: Option<ReportStatusString>) -> Result<Vec<Report>> {
        let filter = if let Some(status) = status {
            doc! { "status": format!("{status:?}") }
        } else {
            doc! {}
        };

        Ok(self
            .col::<Report>(COL)
            .find(filter)
            .await
            .map_err(|_| create_database_error!("find", COL))?
            .filter_map(|s| async {
                if cfg!(debug_assertions) {
                    Some(s.unwrap())
                } else {
                    s.ok()
                }
            })
            .collect()
            .await)
    }

    async fn update_report(&self, id: &str, report: &Report) -> Result<()> {
        self.col::<Report>(COL)
            .replace_one(doc! { "_id": id }, report)
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("replace_one", COL))
    }
}
