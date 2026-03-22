use bson::doc;
use futures::StreamExt;
use revolt_result::Result;

use crate::MongoDb;
use crate::Snapshot;

use super::AbstractSnapshot;

static COL: &str = "safety_snapshots";

#[async_trait]
impl AbstractSnapshot for MongoDb {
    /// Insert a new snapshot into the database
    async fn insert_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        query!(self, insert_one, COL, &snapshot).map(|_| ())
    }

    async fn fetch_snapshots_by_report(&self, report_id: &str) -> Result<Vec<Snapshot>> {
        Ok(self
            .col::<Snapshot>(COL)
            .find(doc! { "report_id": report_id })
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
}
