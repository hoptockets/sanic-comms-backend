use revolt_result::Result;

use crate::Database;
use crate::EmojiParent;

auto_derived!(
    /// Server sticker (image asset, separate from unicode/custom emoji).
    pub struct Sticker {
        /// Unique Id (matches uploaded attachment id)
        #[serde(rename = "_id")]
        pub id: String,
        /// Owning server
        pub parent: EmojiParent,
        /// Uploader user id
        pub creator_id: String,
        /// Sticker name
        pub name: String,
        /// Whether the sticker is animated (GIF)
        #[serde(skip_serializing_if = "crate::if_false", default)]
        pub animated: bool,
        /// Whether the sticker is marked as nsfw
        #[serde(skip_serializing_if = "crate::if_false", default)]
        pub nsfw: bool,
    }
);

impl Sticker {
    fn parent_server_id(&self) -> &str {
        match &self.parent {
            EmojiParent::Server { id } => id,
            EmojiParent::Detached => "",
        }
    }

    /// Persist sticker without fan-out events (clients refetch lists).
    pub async fn create(&self, db: &Database) -> Result<()> {
        db.insert_sticker(self).await?;
        Ok(())
    }

    pub async fn delete(self, db: &Database) -> Result<()> {
        db.detach_sticker(&self).await
    }
}
