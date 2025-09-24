#![allow(clippy::pedantic)]
#![warn(clippy::all)]
use std::collections::{HashSet, HashMap};
use std::path::Path;
use anyhow::{Context, Result};
use log;
use tokio::fs;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use md5;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileExtensionPreset {
    CaptionWdTags,
    FlorenceWdTags,
}
impl fmt::Display for FileExtensionPreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CaptionWdTags => write!(f, "caption+wd+tags"),
            Self::FlorenceWdTags => write!(f, "florence+wd+tags"),
        }
    }
}
/// Configuration for file concatenation
///
/// This configuration controls how files are concatenated, with the following behavior:
/// - Base extensions define which files to look for (e.g., jpg, png)
/// - Extensions to concatenate define which related files to process (e.g., caption, wd, tags)
/// - Caption files (with extension "caption" or "florence") are treated specially:
///   - Their content is appended after the concatenated tags
///   - They aren't included in tag deduplication
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConcatConfig {
    pub base_extensions: Vec<String>,
    pub extensions_to_concat: Vec<String>,
    pub output_extension: String,
    pub remove_duplicates: bool,
    pub tag_separator: String,
    pub deduplicate_files: bool,
}
impl ConcatConfig {
    #[must_use]
    pub fn new(
        base_extensions: Vec<String>,
        extensions_to_concat: Vec<String>,
        output_extension: String,
        remove_duplicates: bool,
        tag_separator: String,
    ) -> Self {
        Self {
            base_extensions,
            extensions_to_concat,
            output_extension,
            remove_duplicates,
            tag_separator,
            deduplicate_files: false,
        }
    }
    #[must_use]
    pub fn with_deduplication(mut self, deduplicate: bool) -> Self {
        self.deduplicate_files = deduplicate;
        self
    }
    #[must_use]
    pub fn from_preset(preset: FileExtensionPreset) -> Self {
        match preset {
            FileExtensionPreset::CaptionWdTags => {
                Self {
                    base_extensions: vec![
                        "png".into(), "jpg".into(), "jpeg".into(), "webp".into(), "gif"
                        .into(), "tiff".into(), "bmp".into(), "jxl".into(), "avif".into()
                    ],
                    extensions_to_concat: vec![
                        "caption".into(), "wd".into(), "tags".into()
                    ],
                    output_extension: "txt".into(),
                    remove_duplicates: true,
                    tag_separator: ", ".into(),
                    deduplicate_files: false,
                }
            }
            FileExtensionPreset::FlorenceWdTags => {
                Self {
                    base_extensions: vec![
                        "png".into(), "jpg".into(), "jpeg".into(), "webp".into(), "gif"
                        .into(), "tiff".into(), "bmp".into(), "jxl".into(), "avif".into()
                    ],
                    extensions_to_concat: vec![
                        "florence".into(), "wd".into(), "tags".into()
                    ],
                    output_extension: "txt".into(),
                    remove_duplicates: true,
                    tag_separator: ", ".into(),
                    deduplicate_files: false,
                }
            }
        }
    }
}
async fn read_file_content(path: &Path) -> Result<String> {
    let content = fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read file: {}", path.display()))?;
    Ok(content.trim().to_string())
}
fn concat_tags(
    contents: &[String],
    config: &ConcatConfig,
    file_paths: &[std::path::PathBuf],
) -> String {
    if contents.is_empty() {
        return String::new();
    }
    let caption_ext = if config.extensions_to_concat.contains(&"caption".to_string()) {
        "caption"
    } else if config.extensions_to_concat.contains(&"florence".to_string()) {
        "florence"
    } else {
        config.extensions_to_concat.last().unwrap()
    };
    let mut caption_index = None;
    for (i, path) in file_paths.iter().enumerate() {
        if let Some(ext) = path.extension() {
            if ext == caption_ext {
                caption_index = Some(i);
                break;
            }
        }
    }
    let caption_index = caption_index.unwrap_or(contents.len() - 1);
    let caption_content = &contents[caption_index];
    let mut unique_tags = HashSet::new();
    let mut all_tags = Vec::new();
    for (i, content) in contents.iter().enumerate() {
        if i == caption_index {
            continue;
        }
        let tags = content.split(',').map(str::trim).filter(|&tag| !tag.is_empty());
        for tag in tags {
            if config.remove_duplicates {
                unique_tags.insert(tag.to_string());
            } else {
                all_tags.push(tag.to_string());
            }
        }
    }
    let tags_portion = if config.remove_duplicates {
        let mut sorted_tags: Vec<_> = unique_tags.into_iter().collect();
        sorted_tags.sort();
        sorted_tags.join(&config.tag_separator)
    } else {
        all_tags.join(&config.tag_separator)
    };
    if tags_portion.is_empty() {
        caption_content.clone()
    } else if caption_content.is_empty() {
        tags_portion
    } else {
        format!("{}{}{}", tags_portion, config.tag_separator, caption_content)
    }
}
pub async fn process_image_file(
    image_path: &Path,
    config: &ConcatConfig,
    dry_run: bool,
) -> Result<bool> {
    let stem = image_path
        .file_stem()
        .with_context(|| {
            format!("Failed to get file stem from: {}", image_path.display())
        })?
        .to_string_lossy();
    let parent = image_path
        .parent()
        .with_context(|| {
            format!("Failed to get parent directory of: {}", image_path.display())
        })?;
    let mut missing_files = Vec::new();
    let mut file_paths = Vec::new();
    for ext in &config.extensions_to_concat {
        let ext_file = parent.join(format!("{stem}.{ext}"));
        if ext_file.exists() {
            file_paths.push(ext_file);
        } else {
            missing_files.push(ext_file.to_string_lossy().to_string());
        }
    }
    if !missing_files.is_empty() {
        log::warn!(
            "Skipping {}: Missing files: {}", image_path.display(), missing_files
            .join(", ")
        );
        return Ok(false);
    }
    let mut contents = Vec::new();
    for path in &file_paths {
        let content = read_file_content(path).await?;
        contents.push(content);
    }
    let concatenated = concat_tags(&contents, config, &file_paths);
    let output_path = parent.join(format!("{}.{}", stem, config.output_extension));
    if dry_run {
        log::info!("Would write to {}: {}", output_path.display(), concatenated);
    } else {
        fs::write(&output_path, &concatenated)
            .await
            .with_context(|| format!("Failed to write to: {}", output_path.display()))?;
        log::debug!("Wrote {}", output_path.display());
    }
    Ok(true)
}
async fn walk_directory<F, Fut>(
    directory: &Path,
    mut callback: F,
) -> Result<()>
where
    F: FnMut(&Path) -> Fut + Send,
    Fut: std::future::Future<Output = Result<()>> + Send,
{
    let mut dirs_to_visit = vec![directory.to_path_buf()];

    while let Some(current_dir) = dirs_to_visit.pop() {
        let mut entries = fs::read_dir(&current_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_dir() {
                dirs_to_visit.push(path);
            } else {
                callback(&path).await?;
            }
        }
    }

    Ok(())
}

pub async fn concat_files(
    directory: &Path,
    config: &ConcatConfig,
    dry_run: bool,
) -> Result<usize> {
    let directory = directory.to_path_buf();
    let config_clone = config.clone();
    log::info!("Searching for files in: {}", directory.display());
    log::info!("Using extensions: {}", config.extensions_to_concat.join(", "));
    log::info!("Output extension: {}", config.output_extension);
    if config.deduplicate_files {
        log::info!("File deduplication enabled - will check for identical file contents");
    }
    let processed_count = Arc::new(AtomicUsize::new(0));
    let skipped_duplicates = Arc::new(AtomicUsize::new(0));
    let mut base_extensions = HashSet::new();
    for ext in &config.base_extensions {
        base_extensions.insert(ext.clone());
        log::debug!("Added base extension: {}", ext);
    }
    let content_hashes: Arc<tokio::sync::Mutex<HashMap<String, String>>> = Arc::new(
        tokio::sync::Mutex::new(HashMap::new()),
    );
    let processed_count_clone = processed_count.clone();
    let skipped_duplicates_clone = skipped_duplicates.clone();
    let content_hashes_clone = content_hashes.clone();
    walk_directory(
            &directory,
            move |path| {
                let path = path.to_path_buf();
                let base_exts = base_extensions.clone();
                let config = config_clone.clone();
                let dry_run = dry_run;
                let count = processed_count_clone.clone();
                let skipped = skipped_duplicates_clone.clone();
                let hashes = content_hashes_clone.clone();
                async move {
                    if let Some(ext) = path.extension() {
                        let ext_str = ext.to_string_lossy().to_lowercase();
                        log::debug!(
                            "Checking file: {} with extension: {}", path.display(),
                            ext_str
                        );
                        log::debug!("Base extensions: {:?}", base_exts);
                        if base_exts.contains(&ext_str) {
                            log::debug!("Found base extension match: {}", path.display());
                            if config.deduplicate_files {
                                log::debug!(
                                    "Checking for duplicate content: {}", path.display()
                                );
                                let is_duplicate = check_duplicate_content(
                                        &path,
                                        &config,
                                        hashes.clone(),
                                    )
                                    .await;
                                if is_duplicate {
                                    log::debug!("Skipping duplicate file: {}", path.display());
                                    skipped.fetch_add(1, Ordering::Relaxed);
                                    return Ok(());
                                }
                                log::debug!(
                                    "File is not a duplicate, proceeding: {}", path.display()
                                );
                            }
                            log::debug!("Processing file: {}", path.display());
                            match process_image_file(&path, &config, dry_run).await {
                                Ok(true) => {
                                    log::debug!("Successfully processed: {}", path.display());
                                    count.fetch_add(1, Ordering::Relaxed);
                                }
                                Ok(false) => {
                                    log::debug!("Skipped due to missing files: {}", path.display());
                                }
                                Err(err) => {
                                    log::warn!("Error processing {}: {}", path.display(), err)
                                }
                            }
                        } else {
                            log::debug!("Skipping non-base extension: {}", path.display());
                        }
                    }
                    Ok(())
                }
            },
        )
        .await?;
    let final_count = processed_count.load(Ordering::Relaxed);
    let final_skipped = skipped_duplicates.load(Ordering::Relaxed);
    if dry_run {
        log::info!("Dry run completed. Would have processed {} files.", final_count);
    } else {
        log::info!("Concatenation completed. Processed {} files.", final_count);
    }
    if config.deduplicate_files {
        log::info!("Skipped {} duplicate files.", final_skipped);
    }
    Ok(final_count)
}
async fn check_duplicate_content(
    path: &Path,
    config: &ConcatConfig,
    hashes: Arc<tokio::sync::Mutex<HashMap<String, String>>>,
) -> bool {
    let Some(stem) = path.file_stem() else {
        log::debug!("Could not get file stem for: {}", path.display());
        return false;
    };
    let stem = stem.to_string_lossy();
    let Some(parent) = path.parent() else {
        log::debug!("Could not get parent directory for: {}", path.display());
        return false;
    };
    log::debug!(
        "Checking duplicate content for file: {} with stem: {}", path.display(), stem
    );
    let mut file_paths = Vec::new();
    for ext in &config.extensions_to_concat {
        let ext_file = parent.join(format!("{stem}.{ext}"));
        if !ext_file.exists() {
            log::debug!("Missing required file: {}", ext_file.display());
            return false;
        }
        log::debug!("Found required file: {}", ext_file.display());
        file_paths.push(ext_file);
    }
    let mut combined_content = String::new();
    for path in &file_paths {
        match fs::read_to_string(path).await {
            Ok(content) => {
                log::debug!("Read content from: {}", path.display());
                combined_content.push_str(&content);
            }
            Err(err) => {
                log::debug!("Failed to read content from {}: {}", path.display(), err);
                return false;
            }
        }
    }
    let content_hash = format!("{:x}", md5::compute(combined_content.as_bytes()));
    log::debug!("Generated hash for {}: {}", path.display(), content_hash);
    let mut hashes_map = hashes.lock().await;
    if let Some(existing_file) = hashes_map.get(&content_hash) {
        log::debug!("Found duplicate content: {} matches {}", path.display(), existing_file);
        true
    } else {
        log::debug!("No duplicate found for {}, storing hash", path.display());
        hashes_map.insert(content_hash, path.to_string_lossy().to_string());
        false
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;
    #[tokio::test]
    async fn test_concat_tags_with_duplicates() -> Result<()> {
        let config = ConcatConfig {
            base_extensions: vec!["jpg".into()],
            extensions_to_concat: vec!["wd".into(), "tags".into(), "caption".into()],
            output_extension: "txt".into(),
            remove_duplicates: true,
            tag_separator: ", ".into(),
            deduplicate_files: false,
        };
        let contents = vec![
            "tag1, tag2, tag3".to_string(), "tag2, tag4, tag5".to_string(),
            "a photo of a person".to_string(),
        ];
        let file_paths = vec![
            std::path::PathBuf::from("test.wd"), std::path::PathBuf::from("test.tags"),
            std::path::PathBuf::from("test.caption"),
        ];
        let result = concat_tags(&contents, &config, &file_paths);
        assert_eq!(result, "tag1, tag2, tag3, tag4, tag5, a photo of a person");
        Ok(())
    }
    #[tokio::test]
    async fn test_concat_tags_without_duplicates() -> Result<()> {
        let config = ConcatConfig {
            base_extensions: vec!["jpg".into()],
            extensions_to_concat: vec!["wd".into(), "tags".into(), "caption".into()],
            output_extension: "txt".into(),
            remove_duplicates: false,
            tag_separator: ", ".into(),
            deduplicate_files: false,
        };
        let contents = vec![
            "tag1, tag2, tag3".to_string(), "tag2, tag4, tag5".to_string(),
            "a photo of a person".to_string(),
        ];
        let file_paths = vec![
            std::path::PathBuf::from("test.wd"), std::path::PathBuf::from("test.tags"),
            std::path::PathBuf::from("test.caption"),
        ];
        let result = concat_tags(&contents, &config, &file_paths);
        assert_eq!(result, "tag1, tag2, tag3, tag2, tag4, tag5, a photo of a person");
        Ok(())
    }
    #[tokio::test]
    async fn test_process_image_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path();
        let image_path = temp_path.join("test.jpg");
        let caption_path = temp_path.join("test.caption");
        let wd_path = temp_path.join("test.wd");
        let tags_path = temp_path.join("test.tags");
        File::create(&image_path).await?.sync_all().await?;
        let mut caption_file = File::create(&caption_path).await?;
        caption_file.write_all(b"caption1, caption2").await?;
        caption_file.sync_all().await?;
        let mut wd_file = File::create(&wd_path).await?;
        wd_file.write_all(b"wd1, wd2").await?;
        wd_file.sync_all().await?;
        let mut tags_file = File::create(&tags_path).await?;
        tags_file.write_all(b"tag1, tag2").await?;
        tags_file.sync_all().await?;
        let config = ConcatConfig {
            base_extensions: vec!["jpg".into()],
            extensions_to_concat: vec!["caption".into(), "wd".into(), "tags".into()],
            output_extension: "txt".into(),
            remove_duplicates: true,
            tag_separator: ", ".into(),
            deduplicate_files: false,
        };
        let processed_dry = process_image_file(&image_path, &config, true).await?;
        assert!(processed_dry);
        assert!(! temp_path.join("test.txt").exists());
        let processed = process_image_file(&image_path, &config, false).await?;
        assert!(processed);
        let output_content = fs::read_to_string(temp_path.join("test.txt")).await?;
        assert_eq!(output_content, "tag1, tag2, wd1, wd2, caption1, caption2");
        Ok(())
    }
    #[tokio::test]
    async fn test_file_deduplication() -> Result<()> {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .is_test(true)
            .try_init();
        log::info!("Starting file deduplication test");
        let temp_dir = tempfile::tempdir()?;
        let temp_path = temp_dir.path();
        let image1_path = temp_path.join("image1.jpg");
        let image2_path = temp_path.join("image2.jpg");
        let image3_path = temp_path.join("image3.jpg");
        let caption1_path = temp_path.join("image1.caption");
        let caption2_path = temp_path.join("image2.caption");
        let caption3_path = temp_path.join("image3.caption");
        let wd1_path = temp_path.join("image1.wd");
        let wd2_path = temp_path.join("image2.wd");
        let wd3_path = temp_path.join("image3.wd");
        let tags1_path = temp_path.join("image1.tags");
        let tags2_path = temp_path.join("image2.tags");
        let tags3_path = temp_path.join("image3.tags");
        log::info!("Creating test files in {}", temp_path.display());
        let mut image1_file = File::create(&image1_path).await?;
        image1_file.write_all(b"test image 1").await?;
        image1_file.sync_all().await?;
        let mut image2_file = File::create(&image2_path).await?;
        image2_file.write_all(b"test image 2").await?;
        image2_file.sync_all().await?;
        let mut image3_file = File::create(&image3_path).await?;
        image3_file.write_all(b"test image 3").await?;
        image3_file.sync_all().await?;
        let mut caption1_file = File::create(&caption1_path).await?;
        caption1_file.write_all(b"a photo of a person").await?;
        caption1_file.sync_all().await?;
        let mut caption2_file = File::create(&caption2_path).await?;
        caption2_file.write_all(b"a photo of a person").await?;
        caption2_file.sync_all().await?;
        let mut caption3_file = File::create(&caption3_path).await?;
        caption3_file.write_all(b"person, portrait, indoor").await?;
        caption3_file.sync_all().await?;
        let mut wd1_file = File::create(&wd1_path).await?;
        wd1_file.write_all(b"masterpiece, digital art").await?;
        wd1_file.sync_all().await?;
        let mut wd2_file = File::create(&wd2_path).await?;
        wd2_file.write_all(b"masterpiece, digital art").await?;
        wd2_file.sync_all().await?;
        let mut wd3_file = File::create(&wd3_path).await?;
        wd3_file.write_all(b"highly detailed, 4k").await?;
        wd3_file.sync_all().await?;
        let tags_content = "tag1, tag2, tag3";
        let mut tags1_file = File::create(&tags1_path).await?;
        tags1_file.write_all(tags_content.as_bytes()).await?;
        tags1_file.sync_all().await?;
        let mut tags2_file = File::create(&tags2_path).await?;
        tags2_file.write_all(tags_content.as_bytes()).await?;
        tags2_file.sync_all().await?;
        let mut tags3_file = File::create(&tags3_path).await?;
        tags3_file.write_all(b"tag4, tag5, tag6").await?;
        tags3_file.sync_all().await?;
        let config = ConcatConfig {
            base_extensions: vec!["jpg".into()],
            extensions_to_concat: vec!["caption".into(), "wd".into(), "tags".into()],
            output_extension: "txt".into(),
            remove_duplicates: true,
            tag_separator: ", ".into(),
            deduplicate_files: true,
        };
        log::info!("Test files created at:");
        log::info!("Image 1: {}", image1_path.display());
        log::info!("Caption 1: {}", caption1_path.display());
        log::info!("WD 1: {}", wd1_path.display());
        log::info!("Tags 1: {}", tags1_path.display());
        let content_hashes: Arc<tokio::sync::Mutex<HashMap<String, String>>> = Arc::new(
            tokio::sync::Mutex::new(HashMap::new()),
        );
        log::info!("Processing first image: {}", image1_path.display());
        let is_duplicate1 = check_duplicate_content(
                &image1_path,
                &config,
                content_hashes.clone(),
            )
            .await;
        assert!(! is_duplicate1, "First image should not be detected as duplicate");
        let processed1 = process_image_file(&image1_path, &config, false).await?;
        assert!(processed1, "First image should be processed successfully");
        log::info!("Processing second image: {}", image2_path.display());
        let is_duplicate2 = check_duplicate_content(
                &image2_path,
                &config,
                content_hashes.clone(),
            )
            .await;
        assert!(is_duplicate2, "Second image should be detected as duplicate");
        log::info!("Processing third image: {}", image3_path.display());
        let is_duplicate3 = check_duplicate_content(
                &image3_path,
                &config,
                content_hashes.clone(),
            )
            .await;
        assert!(! is_duplicate3, "Third image should not be detected as duplicate");
        let processed3 = process_image_file(&image3_path, &config, false).await?;
        assert!(processed3, "Third image should be processed successfully");
        assert!(temp_path.join("image1.txt").exists(), "image1.txt should exist");
        assert!(
            ! temp_path.join("image2.txt").exists(),
            "image2.txt should not exist (duplicate)"
        );
        assert!(temp_path.join("image3.txt").exists(), "image3.txt should exist");
        let output1_content = fs::read_to_string(temp_path.join("image1.txt")).await?;
        let output3_content = fs::read_to_string(temp_path.join("image3.txt")).await?;
        log::info!("Output 1 content: '{}'", output1_content);
        log::info!("Output 3 content: '{}'", output3_content);
        assert!(
            output1_content.contains("tag1, tag2, tag3"),
            "Output for image1 should contain deduplicated tags"
        );
        assert!(
            output1_content.contains("digital art, masterpiece"),
            "Output for image1 should contain wd content (in alphabetical order)"
        );
        assert!(
            output1_content.contains("a photo of a person"),
            "Output for image1 should contain caption content"
        );
        assert!(
            output3_content.contains("tag4, tag5, tag6"),
            "Output for image3 should contain its unique tags content"
        );
        assert!(
            output3_content.contains("4k, highly detailed"),
            "Output for image3 should contain its unique wd content (in alphabetical order)"
        );
        assert!(
            output3_content.contains("person, portrait, indoor"),
            "Output for image3 should contain its unique caption content"
        );
        Ok(())
    }
    #[tokio::test]
    async fn test_concat_tags_caption_handling() -> Result<()> {
        let config = ConcatConfig {
            base_extensions: vec!["jpg".into()],
            extensions_to_concat: vec!["wd".into(), "tags".into(), "caption".into()],
            output_extension: "txt".into(),
            remove_duplicates: true,
            tag_separator: ", ".into(),
            deduplicate_files: false,
        };
        let contents = vec![
            "person, photo".to_string(), "person, indoor, white background".to_string(),
            "a photo of a person".to_string(),
        ];
        let file_paths = vec![
            std::path::PathBuf::from("test.wd"), std::path::PathBuf::from("test.tags"),
            std::path::PathBuf::from("test.caption"),
        ];
        let result = concat_tags(&contents, &config, &file_paths);
        assert_eq!(
            result, "indoor, person, photo, white background, a photo of a person"
        );
        let config = ConcatConfig {
            base_extensions: vec!["jpg".into()],
            extensions_to_concat: vec!["caption".into(), "wd".into(), "tags".into()],
            output_extension: "txt".into(),
            remove_duplicates: true,
            tag_separator: ", ".into(),
            deduplicate_files: false,
        };
        let contents = vec![
            "a photo of a person".to_string(), "person, photo".to_string(),
            "person, indoor, white background".to_string(),
        ];
        let file_paths = vec![
            std::path::PathBuf::from("test.caption"),
            std::path::PathBuf::from("test.wd"), std::path::PathBuf::from("test.tags"),
        ];
        let result = concat_tags(&contents, &config, &file_paths);
        assert_eq!(
            result, "indoor, person, photo, white background, a photo of a person"
        );
        Ok(())
    }
}