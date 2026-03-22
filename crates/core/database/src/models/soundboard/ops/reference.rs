use revolt_result::Result;

use crate::EmojiParent;
use crate::ReferenceDb;
use crate::SoundboardClip;

use super::AbstractSoundboard;

#[async_trait]
impl AbstractSoundboard for ReferenceDb {
    async fn insert_soundboard_clip(&self, clip: &SoundboardClip) -> Result<()> {
        let mut clips = self.soundboard_clips.lock().await;
        if clips.contains_key(&clip.id) {
            Err(create_database_error!("insert", "soundboard_clip"))
        } else {
            clips.insert(clip.id.to_string(), clip.clone());
            Ok(())
        }
    }

    async fn fetch_soundboard_clip(&self, id: &str) -> Result<SoundboardClip> {
        let clips = self.soundboard_clips.lock().await;
        clips
            .get(id)
            .cloned()
            .ok_or_else(|| create_error!(NotFound))
    }

    async fn fetch_soundboard_by_parent_id(&self, parent_id: &str) -> Result<Vec<SoundboardClip>> {
        let clips = self.soundboard_clips.lock().await;
        Ok(clips
            .values()
            .filter(|c| match &c.parent {
                EmojiParent::Server { id } => id == parent_id,
                _ => false,
            })
            .cloned()
            .collect())
    }

    async fn detach_soundboard_clip(&self, clip: &SoundboardClip) -> Result<()> {
        let mut clips = self.soundboard_clips.lock().await;
        if let Some(c) = clips.get_mut(&clip.id) {
            c.parent = EmojiParent::Detached;
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }
}
