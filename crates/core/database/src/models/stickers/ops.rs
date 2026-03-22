use revolt_result::Result;

use crate::Sticker;

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractStickers: Sync + Send {
    async fn insert_sticker(&self, sticker: &Sticker) -> Result<()>;
    async fn fetch_sticker(&self, id: &str) -> Result<Sticker>;
    async fn fetch_stickers_by_parent_id(&self, parent_id: &str) -> Result<Vec<Sticker>>;
    async fn detach_sticker(&self, sticker: &Sticker) -> Result<()>;
}
