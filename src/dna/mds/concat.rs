use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum FileExtensionPreset {
    CaptionWdTags,
    FlorenceWdTags,
}

#[derive(Debug, Clone)]
pub struct ConcatConfig {
    pub deduplicate: bool,
    pub preset: FileExtensionPreset,
}

impl ConcatConfig {
    pub fn from_preset(preset: FileExtensionPreset) -> Self {
        Self {
            deduplicate: false,
            preset,
        }
    }

    pub fn with_deduplication(mut self, deduplicate: bool) -> Self {
        self.deduplicate = deduplicate;
        self
    }
}
