auto_derived!(
    /// Soundboard clip metadata (audio handled by Autumn `soundboard` tag).
    pub struct SoundboardClip {
        #[cfg_attr(feature = "serde", serde(rename = "_id"))]
        pub id: String,
        pub parent: super::EmojiParent,
        pub creator_id: String,
        pub name: String,
    }

    pub struct DataCreateSoundboardClip {
        pub name: String,
        pub parent: super::EmojiParent,
    }
);
