use bson::Document;
use revolt_result::Result;

use crate::MongoDb;
use crate::Sticker;

use super::AbstractStickers;

static COL: &str = "stickers";

#[async_trait]
impl AbstractStickers for MongoDb {
    async fn insert_sticker(&self, sticker: &Sticker) -> Result<()> {
        query!(self, insert_one, COL, &sticker).map(|_| ())
    }

    async fn fetch_sticker(&self, id: &str) -> Result<Sticker> {
        query!(self, find_one_by_id, COL, id)?.ok_or_else(|| create_error!(NotFound))
    }

    async fn fetch_stickers_by_parent_id(&self, parent_id: &str) -> Result<Vec<Sticker>> {
        query!(
            self,
            find,
            COL,
            doc! {
                "parent.id": parent_id
            }
        )
    }

    async fn detach_sticker(&self, sticker: &Sticker) -> Result<()> {
        self.col::<Document>(COL)
            .update_one(
                doc! {
                    "_id": &sticker.id
                },
                doc! {
                    "$set": {
                        "parent": {
                            "type": "Detached"
                        }
                    }
                },
            )
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("update_one", COL))
    }
}
