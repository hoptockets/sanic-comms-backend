use revolt_result::Result;

use crate::Database;
use crate::EmojiParent;

auto_derived!(
    /// Short audio clip for server soundboard.
    pub struct SoundboardClip {
        #[serde(rename = "_id")]
        pub id: String,
        pub parent: EmojiParent,
        pub creator_id: String,
        pub name: String,
    }
);

impl SoundboardClip {
    pub async fn create(&self, db: &Database) -> Result<()> {
        db.insert_soundboard_clip(self).await?;
        Ok(())
    }

    pub async fn delete(self, db: &Database) -> Result<()> {
        db.detach_soundboard_clip(&self).await
    }
}
