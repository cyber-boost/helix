#![warn(clippy::all, clippy::pedantic)]
use anyhow::Context;
use memmap2::Mmap;
use safetensors::SafeTensors;
use serde_json::Value;
use std::{fs::File, path::Path};
use tokio::task;
use log;
pub async fn process_file(path: &Path) -> anyhow::Result<()> {
    log::info!("Processing file: {}", path.display());
    let path = path.to_path_buf();
    task::spawn_blocking(move || -> anyhow::Result<()> {
            let file = File::open(&path)
                .with_context(|| format!("Failed to open file: {}", path.display()))?;
            let mmap = unsafe { Mmap::map(&file) }
                .with_context(|| {
                    format!("Failed to memory map file: {}", path.display())
                })?;
            let (_header_size, metadata) = SafeTensors::read_metadata(&mmap)
                .with_context(|| {
                    format!("Failed to read metadata from file: {}", path.display())
                })?;
            log::info!("Raw metadata: {:?}", metadata);
            let metadata_json: Value = serde_json::to_value(&metadata)
                .context("Failed to convert metadata to JSON value")?;
            let metadata_to_process = if let Some(meta) = metadata_json
                .get("__metadata__")
            {
                if let Some(meta_str) = meta.get("metadata") {
                    if let Some(s) = meta_str.as_str() {
                        serde_json::from_str(s)
                            .unwrap_or(Value::Object(serde_json::Map::new()))
                    } else {
                        Value::Object(serde_json::Map::new())
                    }
                } else {
                    Value::Object(serde_json::Map::new())
                }
            } else {
                Value::Object(serde_json::Map::new())
            };
            let processed_metadata = super::metadata::extract_training_metadata(
                &metadata_to_process,
            );
            let json_path = path.with_extension("metadata.json");
            std::fs::write(
                    &json_path,
                    serde_json::to_string_pretty(&processed_metadata)
                        .context("Failed to serialize metadata to JSON")?,
                )
                .with_context(|| {
                    format!("Failed to write metadata to {}", json_path.display())
                })?;
            if processed_metadata.as_object().is_none_or(serde_json::Map::is_empty) {
                log::info!("No training metadata found in {}", path.display());
            } else {
                log::info!("Wrote metadata to {}", json_path.display());
            }
            Ok(())
        })
        .await?
}
pub fn inspect_state_dict(path: &Path) -> anyhow::Result<Value> {
    let file = File::open(path).context("Failed to open safensor file")?;
    let mmap = unsafe { Mmap::map(&file) }
        .context("Failed to memory map safensor file")?;
    let (_header_size, metadata) = SafeTensors::read_metadata(&mmap)
        .context("Failed to read metadata from safensor file")?;
    let state_dict: Value = serde_json::to_value(&metadata)
        .context("Failed to convert state dictionary to JSON value")?;
    Ok(state_dict)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::TempDir;
    fn create_test_safetensor(dir: &TempDir, metadata: &str) -> anyhow::Result<PathBuf> {
        let file_path = dir.path().join("test.safetensors");
        let mut file = fs::File::create(&file_path)?;
        serde_json::from_str::<serde_json::Value>(metadata)?;
        let header = serde_json::json!(
            { "__metadata__" : { "metadata" : metadata }, "test_tensor" : { "dtype" :
            "F32", "shape" : [1], "data_offsets" : [0, 4] } }
        );
        let header_str = serde_json::to_string(&header)?;
        let header_bytes = header_str.as_bytes();
        let header_size = (header_bytes.len() as u64).to_le_bytes();
        file.write_all(&header_size)?;
        file.write_all(header_bytes)?;
        file.write_all(&0f32.to_le_bytes())?;
        Ok(file_path)
    }
    #[tokio::test]
    async fn test_process_file_with_metadata() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let metadata = r#"{
            "ss_bucket_info": {
                "buckets": {
                    "0": {
                        "resolution": [1280, 800],
                        "count": 78
                    }
                },
                "mean_img_ar_error": 0.0
            }
        }"#;
        let file_path = create_test_safetensor(&temp_dir, metadata)?;
        process_file(&file_path).await?;
        let json_path = file_path.with_extension("metadata.json");
        assert!(json_path.exists());
        let content = fs::read_to_string(json_path)?;
        let json: Value = serde_json::from_str(&content)?;
        assert!(json.get("ss_bucket_info").is_some());
        Ok(())
    }
    #[tokio::test]
    async fn test_process_file_empty_metadata() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = create_test_safetensor(&temp_dir, "{}")?;
        process_file(&file_path).await?;
        let json_path = file_path.with_extension("metadata.json");
        assert!(json_path.exists());
        let content = fs::read_to_string(json_path)?;
        let json: Value = serde_json::from_str(&content)?;
        assert!(json.as_object().unwrap().is_empty());
        Ok(())
    }
    #[tokio::test]
    async fn test_process_file_invalid_path() {
        let result = process_file(Path::new("nonexistent.safetensors")).await;
        assert!(result.is_err());
    }
    #[tokio::test]
    async fn test_process_file_complex_metadata() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let metadata = r#"{
            "ss_network_args": {
                "network_alpha": 128,
                "network_dim": 64,
                "network_module": "networks.lora"
            },
            "ss_tag_frequency": {
                "tag1": 0.8,
                "tag2": 0.5
            },
            "ss_dataset_dirs": [
                "/path/to/dataset1",
                "/path/to/dataset2"
            ]
        }"#;
        let file_path = create_test_safetensor(&temp_dir, metadata)?;
        process_file(&file_path).await?;
        let json_path = file_path.with_extension("metadata.json");
        let content = fs::read_to_string(json_path)?;
        let json: Value = serde_json::from_str(&content)?;
        assert!(json.get("ss_network_args").is_some());
        assert!(json.get("ss_tag_frequency").is_some());
        assert!(json.get("ss_dataset_dirs").is_some());
        Ok(())
    }
    #[tokio::test]
    async fn test_inspect_state_dict() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let metadata = r#"{
            "ss_network_args": {
                "network_alpha": 128,
                "network_dim": 64,
                "network_module": "networks.lora"
            },
            "ss_tag_frequency": {
                "tag1": 0.8,
                "tag2": 0.5
            }
        }"#;
        let file_path = temp_dir.path().join("test_model.safetensors");
        let mut file = fs::File::create(&file_path)?;
        let header = serde_json::json!(
            { "__metadata__" : { "metadata" : metadata }, "lora_up.weight" : { "dtype" :
            "F32", "shape" : [768, 64], "data_offsets" : [0, 196608] },
            "lora_down.weight" : { "dtype" : "F16", "shape" : [64, 768], "data_offsets" :
            [196608, 294912] }, "conv.bias" : { "dtype" : "F32", "shape" : [32],
            "data_offsets" : [294912, 295040] } }
        );
        let header_str = serde_json::to_string(&header)?;
        let header_bytes = header_str.as_bytes();
        let header_size = (header_bytes.len() as u64).to_le_bytes();
        file.write_all(&header_size)?;
        file.write_all(header_bytes)?;
        file.write_all(&vec![0u8; 295040])?;
        let state_dict = inspect_state_dict(&file_path)?;
        assert!(state_dict.is_object());
        let obj = state_dict.as_object().unwrap();
        assert!(obj.contains_key("__metadata__"));
        assert!(obj.contains_key("lora_up.weight"));
        assert!(obj.contains_key("lora_down.weight"));
        assert!(obj.contains_key("conv.bias"));
        let up_weight = obj.get("lora_up.weight").unwrap();
        assert_eq!(up_weight.get("dtype").unwrap().as_str().unwrap(), "F32");
        assert_eq!(
            up_weight.get("shape").unwrap().as_array().unwrap(), &
            [serde_json::json!(768), serde_json::json!(64)]
        );
        let down_weight = obj.get("lora_down.weight").unwrap();
        assert_eq!(down_weight.get("dtype").unwrap().as_str().unwrap(), "F16");
        assert_eq!(
            down_weight.get("shape").unwrap().as_array().unwrap(), &
            [serde_json::json!(64), serde_json::json!(768)]
        );
        let metadata_field = obj.get("__metadata__").unwrap();
        let metadata_content = metadata_field.get("metadata").unwrap().as_str().unwrap();
        assert!(metadata_content.contains("network_alpha"));
        Ok(())
    }
}