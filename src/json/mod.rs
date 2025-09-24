#![warn(clippy::all, clippy::pedantic)]
pub mod core;
pub mod caption;
pub mod metadata;
pub mod reasoning;
pub mod st;
pub mod concat;
mod hf;
use log::info;
// pub use xio; // xio module doesn't exist
use anyhow::{Context, Result};
use serde_json::Value;
use std::{
    io, path::{Path, PathBuf},
    sync::Arc,
};
use memmap2;
#[cfg(test)]
mod tests {
    pub mod e621_tests;
    pub mod text_tests;
}
fn get_json_metadata(path: &Path) -> Result<Value> {
    use safetensors::SafeTensors;
    use memmap2::MmapOptions;
    use std::fs::File;
    let file = File::open(path).context("Failed to open file")?;
    let mmap = unsafe { MmapOptions::new().map(&file).context("Failed to mmap file")? };
    let (_header_size, metadata) = SafeTensors::read_metadata(&mmap)
        .context("Failed to read metadata")?;
    let metadata_json: Value = serde_json::to_value(&metadata)
        .context("Failed to convert metadata to JSON value")?;
    let training_metadata = metadata::extract_training_metadata(&metadata_json);
    Ok(training_metadata)
}
pub async fn process_safetensors_file(path: &Path) -> Result<()> {
    let json = get_json_metadata(path)?;
    let pretty_json = serde_json::to_string_pretty(&json)?;
    info!("{pretty_json}");
    tokio::fs::write(path.with_extension("json"), pretty_json).await?;
    Ok(())
}
pub async fn process_caption_file(path: &Path) -> Result<()> {
    caption::process_file(path).await
}
#[must_use = "Processes a JSON file and requires handling of the result to ensure proper file processing"]
pub async fn process_json_file<F, Fut>(file_path: &Path, processor: F) -> io::Result<()>
where
    F: FnOnce(Value) -> Fut + Send,
    Fut: std::future::Future<Output = io::Result<()>> + Send,
{
    let content = tokio::fs::read_to_string(file_path).await?;
    let data: Value = serde_json::from_str(&content)?;
    processor(data).await
}
#[must_use = "Formats a JSON file and requires handling of the result to ensure the file is properly formatted"]
pub async fn format_json_file(path: PathBuf) -> Result<()> {
    info!("Processing file: {}", path.display());
    let file_content = tokio::fs::read_to_string(path.clone())
        .await
        .context("Failed to read file content")?;
    let json: Value = serde_json::from_str(&file_content)
        .context("Failed to parse JSON")?;
    let pretty_json = serde_json::to_string_pretty(&json)
        .context("Failed to format JSON")?;
    tokio::fs::write(path.clone(), pretty_json)
        .await
        .context("Failed to write formatted JSON")?;
    info!("Formatted {} successfully.", path.display());
    Ok(())
}
#[must_use = "Splits content into tags and sentences and the result should be checked"]
pub fn split_content(content: &str) -> (Vec<String>, String) {
    let split: Vec<_> = content.split("., ").collect();
    let tags: Vec<_> = split[0].split(',').map(str::trim).map(String::from).collect();
    let sentences = (*split.get(1).unwrap_or(&"")).to_string();
    (tags, sentences.trim().to_string())
}
#[must_use = "Processes a JSON file to create a caption file and requires handling of the result to ensure proper conversion"]
pub async fn process_json_to_caption(input_path: &Path) -> io::Result<()> {
    if input_path.extension().and_then(|s| s.to_str()) != Some("json") {
        return Ok(());
    }
    let content = tokio::fs::read_to_string(input_path).await?;
    let json: Value = serde_json::from_str(&content)?;
    info!("Processing JSON: {}", json);
    let mut tags = Vec::new();
    if let Value::Object(map) = json {
        for (tag, prob) in map {
            if let Value::Number(prob) = prob {
                if let Some(prob) = prob.as_f64() {
                    if prob >= 0.2 {
                        tags.push((tag, prob));
                    }
                }
            }
        }
    }
    tags.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let tags: Vec<_> = tags
        .into_iter()
        .map(|(tag, _)| { tag.replace('(', "\\(").replace(')', "\\)") })
        .collect();
    let output = tags.join(", ");
    tokio::fs::write(input_path.with_extension("txt"), output).await?;
    Ok(())
}
#[must_use = "Renames a file and requires handling of the result to ensure the file is properly renamed"]
pub async fn rename_file_without_image_extension(path: &Path) -> io::Result<()> {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid file name",
        ))?;
    let parts: Vec<&str> = file_name.split('.').collect();
    if parts.len() >= 3 {
        let mut has_image_ext = false;
        for ext in &parts[1..parts.len() - 1] {
            if matches!(ext.to_lowercase().as_str(), "jpg" | "jpeg" | "png") {
                has_image_ext = true;
                break;
            }
        }
        if has_image_ext {
            let mut new_name = String::from(parts[0]);
            let last_ext = parts.last().unwrap();
            new_name.push('.');
            new_name.push_str(last_ext);
            let parent = path.parent().unwrap_or_else(|| Path::new(""));
            let new_path = parent.join(new_name);
            tokio::fs::rename(path, &new_path).await?;
            info!("Renamed {} to {}", path.display(), new_path.display());
        }
    }
    Ok(())
}
pub async fn process_e621_json_file(
    file_path: &Path,
    config: Option<caption::E621Config>,
) -> Result<()> {
    let content = tokio::fs::read_to_string(file_path).await?;
    let data_owned: Value = serde_json::from_str(&content)?;
    let file_path = Arc::new(file_path.to_path_buf());
    caption::process_e621_json_data(&data_owned, &file_path, config).await
}
pub use caption::{
    caption_file_exists_and_not_empty, format_text_content, json_to_text, process_file,
    replace_special_chars, replace_string,
};

pub use core::{
    TrainingFormat, TrainingSample, TrainingDataset, DatasetStats,
    BCOSample, BCODataset, DPOSample, DPODataset, PPOSample, PPODataset,
    SFTSample, SFTDataset, DatasetQualityReport,
    GenericJSONDataset,
};

pub use hf::{HfProcessor, HfDatasetConfig, HuggingFaceDataset};