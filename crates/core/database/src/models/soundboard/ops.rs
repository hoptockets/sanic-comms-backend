use revolt_result::Result;

use crate::SoundboardClip;

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractSoundboard: Sync + Send {
    async fn insert_soundboard_clip(&self, clip: &SoundboardClip) -> Result<()>;
    async fn fetch_soundboard_clip(&self, id: &str) -> Result<SoundboardClip>;
    async fn fetch_soundboard_by_parent_id(&self, parent_id: &str) -> Result<Vec<SoundboardClip>>;
    async fn detach_soundboard_clip(&self, clip: &SoundboardClip) -> Result<()>;
}
