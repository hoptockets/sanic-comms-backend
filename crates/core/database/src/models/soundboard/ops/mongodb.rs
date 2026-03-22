use bson::Document;
use revolt_result::Result;

use crate::MongoDb;
use crate::SoundboardClip;

use super::AbstractSoundboard;

static COL: &str = "soundboard_clips";

#[async_trait]
impl AbstractSoundboard for MongoDb {
    async fn insert_soundboard_clip(&self, clip: &SoundboardClip) -> Result<()> {
        query!(self, insert_one, COL, &clip).map(|_| ())
    }

    async fn fetch_soundboard_clip(&self, id: &str) -> Result<SoundboardClip> {
        query!(self, find_one_by_id, COL, id)?.ok_or_else(|| create_error!(NotFound))
    }

    async fn fetch_soundboard_by_parent_id(&self, parent_id: &str) -> Result<Vec<SoundboardClip>> {
        query!(
            self,
            find,
            COL,
            doc! {
                "parent.id": parent_id
            }
        )
    }

    async fn detach_soundboard_clip(&self, clip: &SoundboardClip) -> Result<()> {
        self.col::<Document>(COL)
            .update_one(
                doc! {
                    "_id": &clip.id
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
