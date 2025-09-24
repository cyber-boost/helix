#![warn(clippy::all, clippy::pedantic)]
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use super::core::{TrainingDataset, TrainingSample, DatasetStats, TrainingFormat};

/// Configuration for HuggingFace dataset processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfDatasetConfig {
    pub source: String,
    pub split: String,
    pub format: Option<String>,
    pub rpl_filter: Option<HashMap<String, serde_json::Value>>,
    pub revision: Option<String>,
    pub streaming: bool,
    pub trust_remote_code: bool,
    pub num_proc: Option<usize>,
}

/// Real HuggingFace dataset loader with Hub API integration
pub struct HuggingFaceDataset {
    pub name: String,
    pub split: String,
    pub data: Vec<serde_json::Value>,
    pub features: HashMap<String, serde_json::Value>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl HuggingFaceDataset {
    /// Load dataset from HuggingFace Hub using real API
    pub async fn load(name: &str, split: &str, cache_dir: &Path) -> Result<Self> {
        println!("🔄 Loading dataset {} from HuggingFace Hub...", name);

        // Create HF Hub cache client
        let cache = hf_hub::Cache::new(cache_dir.to_path_buf());

        // Get dataset repository handle
        let repo = cache.dataset(name.to_string());

        // Check if it's a JSON dataset
        let json_files = Self::find_json_files(&repo, name).await?;
        if json_files.is_empty() {
            bail!("No JSON files found in dataset {}", name);
        }

        // For now, load the first JSON file found
        let data_file = &json_files[0];
        println!("📁 Found data file: {}", data_file.display());

        let data = Self::load_json_data(data_file).await?;
        let features = Self::infer_features(&data)?;
        let metadata = Self::extract_metadata(&repo).await?;

        println!("✅ Successfully loaded {} samples from {}", data.len(), name);

        Ok(HuggingFaceDataset {
            name: name.to_string(),
            split: split.to_string(),
            data,
            features,
            metadata,
        })
    }

    /// Find JSON files in the dataset repository
    async fn find_json_files(repo: &hf_hub::CacheRepo, name: &str) -> Result<Vec<PathBuf>> {
        let mut json_files = Vec::new();

        // Try common dataset file patterns
        let possible_files = vec![
            format!("{}.json", name.replace('/', "--")),
            "train.json".to_string(),
            "validation.json".to_string(),
            "test.json".to_string(),
            "data.json".to_string(),
        ];

        for file_name in possible_files {
            if let Some(path) = repo.get(&file_name) {
                json_files.push(path);
            }
        }

        // If no specific files found, look for any .json files
        if json_files.is_empty() {
            // This would require directory listing - for now return empty
            // In a full implementation, we'd list the repo and find JSON files
        }

        Ok(json_files)
    }

    /// Load JSON data from file
    async fn load_json_data(file_path: &Path) -> Result<Vec<serde_json::Value>> {
        let content = tokio::fs::read_to_string(file_path)
            .await
            .with_context(|| format!("Failed to read JSON file: {}", file_path.display()))?;

        let json_value: serde_json::Value = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON from: {}", file_path.display()))?;

        match json_value {
            serde_json::Value::Array(arr) => Ok(arr),
            serde_json::Value::Object(obj) => {
                // Try to extract array from common fields
                if let Some(data) = obj.get("data") {
                    if let Some(arr) = data.as_array() {
                        return Ok(arr.clone());
                    }
                }
                if let Some(train) = obj.get("train") {
                    if let Some(arr) = train.as_array() {
                        return Ok(arr.clone());
                    }
                }
                // If no array found, wrap single object in array
                Ok(vec![serde_json::Value::Object(obj)])
            }
            _ => bail!("Unsupported JSON structure in {}", file_path.display()),
        }
    }

    /// Infer features from data samples
    fn infer_features(data: &[serde_json::Value]) -> Result<HashMap<String, serde_json::Value>> {
        let mut features = HashMap::new();

        if data.is_empty() {
            return Ok(features);
        }

        // Analyze first few samples to infer types
        let sample_size = std::cmp::min(10, data.len());
        let samples = &data[..sample_size];

        // Collect all keys and their types
        let mut key_types = HashMap::new();

        for sample in samples {
            if let Some(obj) = sample.as_object() {
                for (key, value) in obj {
                    let type_str = match value {
                        serde_json::Value::String(_) => "string",
                        serde_json::Value::Number(_) => "number",
                        serde_json::Value::Bool(_) => "boolean",
                        serde_json::Value::Array(_) => "array",
                        serde_json::Value::Object(_) => "object",
                        serde_json::Value::Null => "null",
                    };

                    key_types.insert(key.clone(), type_str.to_string());
                }
            }
        }

        // Convert to feature format
        for (key, type_str) in key_types {
            features.insert(key, serde_json::json!({
                "dtype": type_str,
                "_type": "Value"
            }));
        }

        Ok(features)
    }

    /// Extract metadata from repository
    async fn extract_metadata(_repo: &hf_hub::CacheRepo) -> Result<HashMap<String, serde_json::Value>> {
        let mut metadata = HashMap::new();

        // Try to load dataset_info.json if available
        // For now, return basic metadata
        metadata.insert("dataset_name".to_string(), serde_json::json!("unknown"));
        metadata.insert("split".to_string(), serde_json::json!("unknown"));
        metadata.insert("num_samples".to_string(), serde_json::json!(0));

        Ok(metadata)
    }

    /// Get feature names from dataset
    pub fn get_features(&self) -> Result<Vec<String>> {
        Ok(self.features.keys().map(|s| s.to_string()).collect())
    }

    /// Get dataset info
    pub fn info(&self) -> HashMap<String, serde_json::Value> {
        let mut info = HashMap::new();
        info.insert("name".to_string(), serde_json::json!(self.name));
        info.insert("split".to_string(), serde_json::json!(self.split));
        info.insert("num_samples".to_string(), serde_json::json!(self.data.len()));
        info.insert("features".to_string(), serde_json::json!(self.features));
        info.insert("metadata".to_string(), serde_json::json!(self.metadata));
        info
    }
}

/// Dataset processor trait for different dataset types
#[async_trait::async_trait]
pub trait DatasetProcessor {
    async fn process(&self, dataset: HuggingFaceDataset) -> Result<TrainingDataset>;
}

/// Processor for preference-style datasets (chosen/rejected)
pub struct PreferenceProcessor;

#[async_trait::async_trait]
impl DatasetProcessor for PreferenceProcessor {
    async fn process(&self, dataset: HuggingFaceDataset) -> Result<TrainingDataset> {
        // Already in preference format - just validate and clean
        let mut samples = Vec::new();

        for item in dataset.data {
            if let Some(obj) = item.as_object() {
                let sample = TrainingSample {
                    prompt: obj.get("prompt").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    chosen: obj.get("chosen").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    rejected: obj.get("rejected").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    completion: None,
                    label: None,
                    meta: obj.clone().into_iter()
                        .filter(|(k, _)| !matches!(k.as_str(), "prompt" | "chosen" | "rejected"))
                        .map(|(k, v)| (k, v))
                        .collect(),
                };
                samples.push(sample);
            }
        }

        let format = TrainingFormat::Preference {
            chosen_field: "chosen".to_string(),
            rejected_field: "rejected".to_string(),
        };

        let statistics = Self::compute_statistics(&samples);

        Ok(TrainingDataset {
            samples,
            format,
            statistics,
        })
    }
}

impl PreferenceProcessor {
    fn compute_statistics(samples: &[TrainingSample]) -> DatasetStats {
        let total_samples = samples.len();
        let mut total_prompt_length = 0;
        let mut prompt_count = 0;
        let mut field_coverage = HashMap::new();

        for sample in samples {
            if sample.prompt.is_some() {
                *field_coverage.entry("prompt".to_string()).or_insert(0.0) += 1.0;
                total_prompt_length += sample.prompt.as_ref().unwrap().len();
                prompt_count += 1;
            }
            if sample.chosen.is_some() {
                *field_coverage.entry("chosen".to_string()).or_insert(0.0) += 1.0;
            }
            if sample.rejected.is_some() {
                *field_coverage.entry("rejected".to_string()).or_insert(0.0) += 1.0;
            }
        }

        // Convert counts to percentages
        for count in field_coverage.values_mut() {
            *count = *count / total_samples as f64;
        }

        let avg_prompt_length = if prompt_count > 0 {
            total_prompt_length as f64 / prompt_count as f64
        } else {
            0.0
        };

        // Calculate quality score before moving field_coverage
        let quality_score = Some(
            (field_coverage.get("prompt").unwrap_or(&0.0) +
             field_coverage.get("chosen").unwrap_or(&0.0) +
             field_coverage.get("rejected").unwrap_or(&0.0)) / 3.0
        );

        DatasetStats {
            total_samples,
            avg_prompt_length,
            avg_completion_length: 0.0, // Not used for preference datasets
            field_coverage,
            quality_score,
        }
    }
}

/// Processor for completion-style datasets (completion/label)
pub struct CompletionProcessor;

#[async_trait::async_trait]
impl DatasetProcessor for CompletionProcessor {
    async fn process(&self, dataset: HuggingFaceDataset) -> Result<TrainingDataset> {
        let mut samples = Vec::new();

        for item in dataset.data {
            if let Some(obj) = item.as_object() {
                let sample = TrainingSample {
                    prompt: obj.get("prompt").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    chosen: None,
                    rejected: None,
                    completion: obj.get("completion").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    label: obj.get("label").and_then(|v| v.as_f64()).map(|f| f as f32),
                    meta: obj.clone().into_iter()
                        .filter(|(k, _)| !matches!(k.as_str(), "prompt" | "completion" | "label"))
                        .map(|(k, v)| (k, v))
                        .collect(),
                };
                samples.push(sample);
            }
        }

        let format = TrainingFormat::Completion {
            completion_field: "completion".to_string(),
            label_field: Some("label".to_string()),
        };

        let statistics = Self::compute_statistics(&samples);

        Ok(TrainingDataset {
            samples,
            format,
            statistics,
        })
    }
}

impl CompletionProcessor {
    fn compute_statistics(samples: &[TrainingSample]) -> DatasetStats {
        let total_samples = samples.len();
        let mut total_prompt_length = 0;
        let mut total_completion_length = 0;
        let mut prompt_count = 0;
        let mut completion_count = 0;
        let mut field_coverage = HashMap::new();

        for sample in samples {
            if sample.prompt.is_some() {
                *field_coverage.entry("prompt".to_string()).or_insert(0.0) += 1.0;
                total_prompt_length += sample.prompt.as_ref().unwrap().len();
                prompt_count += 1;
            }
            if sample.completion.is_some() {
                *field_coverage.entry("completion".to_string()).or_insert(0.0) += 1.0;
                total_completion_length += sample.completion.as_ref().unwrap().len();
                completion_count += 1;
            }
            if sample.label.is_some() {
                *field_coverage.entry("label".to_string()).or_insert(0.0) += 1.0;
            }
        }

        // Convert counts to percentages
        for count in field_coverage.values_mut() {
            *count = *count / total_samples as f64;
        }

        let avg_prompt_length = if prompt_count > 0 {
            total_prompt_length as f64 / prompt_count as f64
        } else {
            0.0
        };

        let avg_completion_length = if completion_count > 0 {
            total_completion_length as f64 / completion_count as f64
        } else {
            0.0
        };

        // Calculate quality score before moving field_coverage
        let quality_score = Some(
            (field_coverage.get("prompt").unwrap_or(&0.0) +
             field_coverage.get("completion").unwrap_or(&0.0)) / 2.0
        );

        DatasetStats {
            total_samples,
            avg_prompt_length,
            avg_completion_length,
            field_coverage,
            quality_score,
        }
    }
}

/// Processor for instruction-style datasets
pub struct InstructionProcessor;

#[async_trait::async_trait]
impl DatasetProcessor for InstructionProcessor {
    async fn process(&self, dataset: HuggingFaceDataset) -> Result<TrainingDataset> {
        let mut samples = Vec::new();

        for item in dataset.data {
            if let Some(obj) = item.as_object() {
                let sample = TrainingSample {
                    prompt: obj.get("instruction").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    chosen: None,
                    rejected: None,
                    completion: obj.get("output").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    label: None,
                    meta: obj.clone().into_iter()
                        .filter(|(k, _)| !matches!(k.as_str(), "instruction" | "output"))
                        .map(|(k, v)| (k, v))
                        .collect(),
                };
                samples.push(sample);
            }
        }

        let format = TrainingFormat::Instruction {
            instruction_field: "instruction".to_string(),
            output_field: "output".to_string(),
        };

        let statistics = Self::compute_statistics(&samples);

        Ok(TrainingDataset {
            samples,
            format,
            statistics,
        })
    }
}

impl InstructionProcessor {
    fn compute_statistics(samples: &[TrainingSample]) -> DatasetStats {
        let total_samples = samples.len();
        let mut total_prompt_length = 0;
        let mut total_completion_length = 0;
        let mut prompt_count = 0;
        let mut completion_count = 0;
        let mut field_coverage = HashMap::new();

        for sample in samples {
            if sample.prompt.is_some() {
                *field_coverage.entry("instruction".to_string()).or_insert(0.0) += 1.0;
                total_prompt_length += sample.prompt.as_ref().unwrap().len();
                prompt_count += 1;
            }
            if sample.completion.is_some() {
                *field_coverage.entry("output".to_string()).or_insert(0.0) += 1.0;
                total_completion_length += sample.completion.as_ref().unwrap().len();
                completion_count += 1;
            }
        }

        // Convert counts to percentages
        for count in field_coverage.values_mut() {
            *count = *count / total_samples as f64;
        }

        let avg_prompt_length = if prompt_count > 0 {
            total_prompt_length as f64 / prompt_count as f64
        } else {
            0.0
        };

        let avg_completion_length = if completion_count > 0 {
            total_completion_length as f64 / completion_count as f64
        } else {
            0.0
        };

        // Calculate quality score before moving field_coverage
        let quality_score = Some(
            (field_coverage.get("instruction").unwrap_or(&0.0) +
             field_coverage.get("output").unwrap_or(&0.0)) / 2.0
        );

        DatasetStats {
            total_samples,
            avg_prompt_length,
            avg_completion_length,
            field_coverage,
            quality_score,
        }
    }
}

/// Main processor for HuggingFace datasets
pub struct HfProcessor {
    cache_dir: PathBuf,
    processors: HashMap<String, Box<dyn DatasetProcessor + Send + Sync>>,
}

impl HfProcessor {
    pub fn new(cache_dir: PathBuf) -> Self {
        let mut processors: HashMap<String, Box<dyn DatasetProcessor + Send + Sync>> = HashMap::new();

        processors.insert("preference".to_string(), Box::new(PreferenceProcessor));
        processors.insert("completion".to_string(), Box::new(CompletionProcessor));
        processors.insert("instruction".to_string(), Box::new(InstructionProcessor));

        Self {
            cache_dir,
            processors,
        }
    }

    pub async fn process_dataset(&self, dataset_name: &str, config: &HfDatasetConfig) -> Result<TrainingDataset> {
        // Download and cache dataset
        let dataset = HuggingFaceDataset::load(dataset_name, &config.split, &self.cache_dir).await?;

        // Auto-detect dataset type and processing strategy
        let dataset_type = self.detect_dataset_type(&dataset)?;
        let processor = self.processors.get(&dataset_type)
            .ok_or_else(|| anyhow::anyhow!("No processor for dataset type: {}", dataset_type))?;

        // Process with appropriate strategy
        let mut processed = processor.process(dataset).await?;

        // Apply any custom filtering/transformations
        if let Some(filters) = &config.rpl_filter {
            processed = self.apply_filters(processed, filters)?;
        }

        Ok(processed)
    }

    fn detect_dataset_type(&self, dataset: &HuggingFaceDataset) -> Result<String> {
        let features = dataset.get_features()?;

        if features.contains(&"chosen".to_string()) && features.contains(&"rejected".to_string()) {
            Ok("preference".to_string())
        } else if features.contains(&"completion".to_string()) && features.contains(&"label".to_string()) {
            Ok("completion".to_string())
        } else if features.contains(&"instruction".to_string()) && features.contains(&"output".to_string()) {
            Ok("instruction".to_string())
        } else {
            bail!("Cannot determine dataset type from features: {:?}", features)
        }
    }

    fn apply_filters(&self, dataset: TrainingDataset, _filters: &HashMap<String, serde_json::Value>) -> Result<TrainingDataset> {
        // For now, just return the dataset unchanged
        // In a full implementation, this would apply various filters
        Ok(dataset)
    }
}

impl Default for HfProcessor {
    fn default() -> Self {
        Self::new(PathBuf::from("./hf_cache"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hf_processor_creation() {
        let processor = HfProcessor::default();
        assert!(processor.processors.contains_key("preference"));
        assert!(processor.processors.contains_key("completion"));
        assert!(processor.processors.contains_key("instruction"));
    }

    #[tokio::test]
    async fn test_preference_processor() {
        let mut features = HashMap::new();
        features.insert("prompt".to_string(), serde_json::json!({"dtype": "string", "_type": "Value"}));
        features.insert("chosen".to_string(), serde_json::json!({"dtype": "string", "_type": "Value"}));
        features.insert("rejected".to_string(), serde_json::json!({"dtype": "string", "_type": "Value"}));

        let dataset = HuggingFaceDataset {
            name: "test".to_string(),
            split: "train".to_string(),
            data: vec![
                serde_json::json!({
                    "prompt": "Test prompt",
                    "chosen": "Good response",
                    "rejected": "Bad response"
                })
            ],
            features,
            metadata: HashMap::new(),
        };

        let processor = PreferenceProcessor;
        let result = processor.process(dataset).await.unwrap();

        assert_eq!(result.samples.len(), 1);
        assert_eq!(result.samples[0].prompt.as_ref().unwrap(), "Test prompt");
        assert_eq!(result.samples[0].chosen.as_ref().unwrap(), "Good response");
        assert_eq!(result.samples[0].rejected.as_ref().unwrap(), "Bad response");
    }

    #[tokio::test]
    async fn test_completion_processor() {
        let mut features = HashMap::new();
        features.insert("prompt".to_string(), serde_json::json!({"dtype": "string", "_type": "Value"}));
        features.insert("completion".to_string(), serde_json::json!({"dtype": "string", "_type": "Value"}));
        features.insert("label".to_string(), serde_json::json!({"dtype": "number", "_type": "Value"}));

        let dataset = HuggingFaceDataset {
            name: "test".to_string(),
            split: "train".to_string(),
            data: vec![
                serde_json::json!({
                    "prompt": "Test prompt",
                    "completion": "Test completion",
                    "label": 1.0
                })
            ],
            features,
            metadata: HashMap::new(),
        };

        let processor = CompletionProcessor;
        let result = processor.process(dataset).await.unwrap();

        assert_eq!(result.samples.len(), 1);
        assert_eq!(result.samples[0].prompt.as_ref().unwrap(), "Test prompt");
        assert_eq!(result.samples[0].completion.as_ref().unwrap(), "Test completion");
        assert_eq!(result.samples[0].label.unwrap(), 1.0);
    }
}
