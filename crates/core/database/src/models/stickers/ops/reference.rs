use revolt_result::Result;

use crate::EmojiParent;
use crate::ReferenceDb;
use crate::Sticker;

use super::AbstractStickers;

#[async_trait]
impl AbstractStickers for ReferenceDb {
    async fn insert_sticker(&self, sticker: &Sticker) -> Result<()> {
        let mut stickers = self.stickers.lock().await;
        if stickers.contains_key(&sticker.id) {
            Err(create_database_error!("insert", "sticker"))
        } else {
            stickers.insert(sticker.id.to_string(), sticker.clone());
            Ok(())
        }
    }

    async fn fetch_sticker(&self, id: &str) -> Result<Sticker> {
        let stickers = self.stickers.lock().await;
        stickers
            .get(id)
            .cloned()
            .ok_or_else(|| create_error!(NotFound))
    }

    async fn fetch_stickers_by_parent_id(&self, parent_id: &str) -> Result<Vec<Sticker>> {
        let stickers = self.stickers.lock().await;
        Ok(stickers
            .values()
            .filter(|s| match &s.parent {
                EmojiParent::Server { id } => id == parent_id,
                _ => false,
            })
            .cloned()
            .collect())
    }

    async fn detach_sticker(&self, sticker: &Sticker) -> Result<()> {
        let mut stickers = self.stickers.lock().await;
        if let Some(s) = stickers.get_mut(&sticker.id) {
            s.parent = EmojiParent::Detached;
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }
}
