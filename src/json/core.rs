use anyhow::{bail, Context, Result};
use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use walkdir::WalkDir;
#[derive(Debug, Clone, PartialEq)]
pub enum DataFormat {
    Auto,
    Legacy,
    Molds,
    Custom,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrainingFormat {
    Preference { chosen_field: String, rejected_field: String },
    Completion { completion_field: String, label_field: Option<String> },
    Instruction { instruction_field: String, output_field: String },
    Chat { messages_field: String },
    Custom { fields: Vec<String> },
}
impl std::str::FromStr for DataFormat {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(DataFormat::Auto),
            "legacy" => Ok(DataFormat::Legacy),
            "molds" => Ok(DataFormat::Molds),
            "custom" => Ok(DataFormat::Custom),
            _ => bail!("Invalid format: {}. Must be auto, legacy, molds, or custom", s),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacySample {
    pub x: Vec<f32>,
    pub y: Vec<f32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoldsSample {
    pub module_name: String,
    pub file_name: String,
    pub implementation: String,
    pub documentation: String,
    #[serde(rename = "system_context")]
    pub system_context: Option<String>,
}

/// Universal training sample - works for all algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSample {
    pub prompt: Option<String>,
    pub chosen: Option<String>,      // For DPO, IPO, etc.
    pub rejected: Option<String>,    // For DPO, IPO, etc.
    pub completion: Option<String>,  // For BCO, SFT, etc.
    pub label: Option<f32>,          // For classification tasks
    pub meta: HashMap<String, Value>, // Additional metadata
}

/// Algorithm-agnostic training dataset
#[derive(Debug, Clone)]
pub struct TrainingDataset {
    pub samples: Vec<TrainingSample>,
    pub format: TrainingFormat,
    pub statistics: DatasetStats,
}

/// Dataset statistics for quality assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetStats {
    pub total_samples: usize,
    pub avg_prompt_length: f64,
    pub avg_completion_length: f64,
    pub field_coverage: HashMap<String, f64>,
    pub quality_score: Option<f64>,
}
#[derive(Debug)]
pub struct GenericJSONDataset {
    pub data: Vec<Value>,
    pub format: DataFormat,
    pub schema: Option<Value>,
}
impl GenericJSONDataset {
    pub fn new(
        json_paths: &[PathBuf],
        schema_path: Option<&Path>,
        data_format: DataFormat,
    ) -> Result<Self> {
        if json_paths.is_empty() {
            bail!("No JSON files provided");
        }
        for path in json_paths {
            if !path.is_file() {
                bail!("JSON file not found: {}", path.display());
            }
        }
        let mut raw_data = Vec::new();
        for path in json_paths {
            let content = fs::read_to_string(path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            let parsed: Value = serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse JSON in {}", path.display()))?;
            match parsed {
                Value::Array(arr) => raw_data.extend(arr),
                Value::Object(obj) => raw_data.push(Value::Object(obj)),
                _ => {
                    bail!(
                        "Root object in {} must be an array or object, got {}", path
                        .display(), parsed
                    )
                }
            }
        }
        if raw_data.is_empty() {
            bail!("All input files are empty");
        }
        let format = match data_format {
            DataFormat::Auto => Self::detect_format(&raw_data[0])?,
            _ => data_format,
        };
        let schema = if let Some(schema_path) = schema_path {
            let schema_content = fs::read_to_string(schema_path)
                .with_context(|| {
                    format!("Failed to read schema {}", schema_path.display())
                })?;
            Some(
                serde_json::from_str(&schema_content)
                    .with_context(|| format!("Failed to parse schema JSON"))?,
            )
        } else {
            Self::get_builtin_schema(&format)
        };
        if let Some(ref schema) = schema {
            Self::validate_data(&raw_data, schema, &format)?;
        }
        Ok(GenericJSONDataset {
            data: raw_data,
            format,
            schema,
        })
    }
    fn detect_format(first_sample: &Value) -> Result<DataFormat> {
        if let Some(obj) = first_sample.as_object() {
            if obj.contains_key("module_name") {
                Ok(DataFormat::Molds)
            } else if obj.contains_key("x") && obj.contains_key("y") {
                Ok(DataFormat::Legacy)
            } else {
                Ok(DataFormat::Custom)
            }
        } else {
            bail!("First sample is not an object - cannot auto-detect format");
        }
    }
    fn get_builtin_schema(format: &DataFormat) -> Option<Value> {
        match format {
            DataFormat::Legacy => {
                Some(
                    json!(
                        { "type" : "array", "items" : { "type" : "object", "required" :
                        ["x", "y"] } }
                    ),
                )
            }
            DataFormat::Molds => {
                Some(
                    json!(
                        { "type" : "array", "items" : { "type" : "object", "required" :
                        ["module_name", "file_name", "implementation", "documentation"] }
                        }
                    ),
                )
            }
            _ => None,
        }
    }
    fn validate_data(
        data: &[Value],
        _schema: &Value,
        format: &DataFormat,
    ) -> Result<()> {
        let required_keys = match format {
            DataFormat::Legacy => vec!["x", "y"],
            DataFormat::Molds => {
                vec!["module_name", "file_name", "implementation", "documentation"]
            }
            _ => return Ok(()),
        };
        for (i, sample) in data.iter().enumerate() {
            if let Some(obj) = sample.as_object() {
                for key in &required_keys {
                    if !obj.contains_key(*key) {
                        bail!(
                            "Sample {} is missing required key '{}' for {} format", i,
                            key, format!("{:?}", format) .to_lowercase()
                        );
                    }
                }
            } else {
                bail!("Sample {} is not an object", i);
            }
        }
        Ok(())
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    pub fn get_random_sample(&self) -> Option<&Value> {
        if self.data.is_empty() {
            None
        } else {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let idx = rng.gen_range(0..self.data.len());
            Some(&self.data[idx])
        }
    }
    pub fn stats(&self) -> HashMap<String, Value> {
        let mut stats = HashMap::new();
        stats.insert("num_samples".to_string(), json!(self.len()));
        stats
            .insert(
                "format".to_string(),
                json!(format!("{:?}", self.format) .to_lowercase()),
            );
        stats.insert("has_schema".to_string(), json!(self.schema.is_some()));
        if !self.data.is_empty() {
            if let Some(obj) = self.data[0].as_object() {
                stats
                    .insert(
                        "sample_keys".to_string(),
                        json!(obj.keys().collect::< Vec < _ >> ()),
                    );
            }
        }
        stats
    }

    /// Detect training format from dataset fields
    pub fn detect_training_format(&self) -> Result<TrainingFormat> {
        if self.data.is_empty() {
            bail!("Cannot detect training format from empty dataset");
        }

        let first_sample = &self.data[0];
        let fields = if let Some(obj) = first_sample.as_object() {
            obj.keys().map(|s| s.as_str()).collect::<Vec<_>>()
        } else {
            bail!("First sample is not an object");
        };

        // Check for preference format (chosen/rejected)
        if fields.contains(&"chosen") && fields.contains(&"rejected") {
            return Ok(TrainingFormat::Preference {
                chosen_field: "chosen".to_string(),
                rejected_field: "rejected".to_string(),
            });
        }

        // Check for completion format (completion/label)
        if fields.contains(&"completion") {
            let label_field = if fields.contains(&"label") {
                Some("label".to_string())
            } else {
                None
            };
            return Ok(TrainingFormat::Completion {
                completion_field: "completion".to_string(),
                label_field,
            });
        }

        // Check for instruction format
        if fields.contains(&"instruction") && fields.contains(&"output") {
            return Ok(TrainingFormat::Instruction {
                instruction_field: "instruction".to_string(),
                output_field: "output".to_string(),
            });
        }

        // Check for chat format
        if fields.contains(&"messages") {
            return Ok(TrainingFormat::Chat {
                messages_field: "messages".to_string(),
            });
        }

        // Fall back to custom format
        Ok(TrainingFormat::Custom {
            fields: fields.into_iter().map(|s| s.to_string()).collect(),
        })
    }

    /// Convert to universal TrainingDataset format
    pub fn to_training_dataset(&self) -> Result<TrainingDataset> {
        let training_format = self.detect_training_format()?;
        let mut samples = Vec::new();

        for (i, sample) in self.data.iter().enumerate() {
            if let Some(obj) = sample.as_object() {
                let training_sample = self.convert_sample_to_training(obj, &training_format)
                    .with_context(|| format!("Failed to convert sample {}", i))?;
                samples.push(training_sample);
            } else {
                bail!("Sample {} is not an object", i);
            }
        }

        let statistics = self.compute_statistics(&samples)?;

        Ok(TrainingDataset {
            samples,
            format: training_format,
            statistics,
        })
    }

    fn convert_sample_to_training(&self, obj: &serde_json::Map<String, Value>, format: &TrainingFormat) -> Result<TrainingSample> {
        let mut sample = TrainingSample {
            prompt: None,
            chosen: None,
            rejected: None,
            completion: None,
            label: None,
            meta: HashMap::new(),
        };

        // Extract prompt if available
        if let Some(prompt_val) = obj.get("prompt") {
            if let Some(prompt_str) = prompt_val.as_str() {
                sample.prompt = Some(prompt_str.to_string());
            }
        }

        match format {
            TrainingFormat::Preference { chosen_field, rejected_field } => {
                if let Some(chosen_val) = obj.get(chosen_field) {
                    if let Some(chosen_str) = chosen_val.as_str() {
                        sample.chosen = Some(chosen_str.to_string());
                    }
                }
                if let Some(rejected_val) = obj.get(rejected_field) {
                    if let Some(rejected_str) = rejected_val.as_str() {
                        sample.rejected = Some(rejected_str.to_string());
                    }
                }
            }
            TrainingFormat::Completion { completion_field, label_field } => {
                if let Some(completion_val) = obj.get(completion_field) {
                    if let Some(completion_str) = completion_val.as_str() {
                        sample.completion = Some(completion_str.to_string());
                    }
                }
                if let Some(label_field) = label_field {
                    if let Some(label_val) = obj.get(label_field) {
                        if let Some(label_num) = label_val.as_f64() {
                            sample.label = Some(label_num as f32);
                        } else if let Some(label_bool) = label_val.as_bool() {
                            sample.label = Some(if label_bool { 1.0 } else { 0.0 });
                        }
                    }
                }
            }
            TrainingFormat::Instruction { instruction_field, output_field } => {
                if let Some(instruction_val) = obj.get(instruction_field) {
                    if let Some(instruction_str) = instruction_val.as_str() {
                        sample.prompt = Some(instruction_str.to_string());
                    }
                }
                if let Some(output_val) = obj.get(output_field) {
                    if let Some(output_str) = output_val.as_str() {
                        sample.completion = Some(output_str.to_string());
                    }
                }
            }
            TrainingFormat::Chat { messages_field } => {
                // For chat format, we'll store the entire messages array in meta
                if let Some(messages_val) = obj.get(messages_field) {
                    sample.meta.insert("messages".to_string(), messages_val.clone());
                    // Try to extract prompt from first message
                    if let Some(messages) = messages_val.as_array() {
                        if let Some(first_msg) = messages.first() {
                            if let Some(content) = first_msg.get("content").and_then(|c| c.as_str()) {
                                sample.prompt = Some(content.to_string());
                            }
                        }
                        // Try to extract completion from last message
                        if let Some(last_msg) = messages.last() {
                            if let Some(content) = last_msg.get("content").and_then(|c| c.as_str()) {
                                sample.completion = Some(content.to_string());
                            }
                        }
                    }
                }
            }
            TrainingFormat::Custom { fields } => {
                // For custom format, store all fields in meta
                for field in fields {
                    if let Some(value) = obj.get(field) {
                        sample.meta.insert(field.clone(), value.clone());
                    }
                }
            }
        }

        // Store any remaining fields in meta
        for (key, value) in obj {
            if !matches!(key.as_str(),
                "prompt" | "chosen" | "rejected" | "completion" | "label" |
                "instruction" | "output" | "messages") {
                sample.meta.insert(key.clone(), value.clone());
            }
        }

        Ok(sample)
    }

    fn compute_statistics(&self, samples: &[TrainingSample]) -> Result<DatasetStats> {
        let total_samples = samples.len();
        let mut total_prompt_length = 0;
        let mut total_completion_length = 0;
        let mut prompt_count = 0;
        let mut completion_count = 0;
        let mut field_coverage = HashMap::new();

        // Count field coverage
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

        // Simple quality score based on field coverage and content
        let quality_score = Some(
            (field_coverage.get("prompt").unwrap_or(&0.0) +
             field_coverage.get("chosen").unwrap_or(&0.0) +
             field_coverage.get("rejected").unwrap_or(&0.0) +
             field_coverage.get("completion").unwrap_or(&0.0)) / 4.0
        );

        Ok(DatasetStats {
            total_samples,
            avg_prompt_length,
            avg_completion_length,
            field_coverage,
            quality_score,
        })
    }
}

impl TrainingDataset {
    /// Convert any format to algorithm-specific format
    pub fn to_algorithm_format(&self, algorithm: &str) -> Result<Box<dyn std::any::Any>> {
        match algorithm.to_lowercase().as_str() {
            "bco" => {
                let bco_data = self.to_bco_format()?;
                Ok(Box::new(bco_data))
            }
            "dpo" => {
                let dpo_data = self.to_dpo_format()?;
                Ok(Box::new(dpo_data))
            }
            "ppo" => {
                let ppo_data = self.to_ppo_format()?;
                Ok(Box::new(ppo_data))
            }
            "sft" => {
                let sft_data = self.to_sft_format()?;
                Ok(Box::new(sft_data))
            }
            _ => bail!("Unsupported algorithm: {}", algorithm),
        }
    }

    fn to_bco_format(&self) -> Result<BCODataset> {
        let mut bco_samples = Vec::new();

        for sample in &self.samples {
            match &self.format {
                TrainingFormat::Preference { .. } => {
                    // Convert preference to completion format for BCO
                    if let (Some(chosen), Some(rejected)) = (&sample.chosen, &sample.rejected) {
                        // Randomly choose between chosen and rejected for BCO
                        let (completion, label) = if rand::random::<bool>() {
                            (chosen.clone(), true)
                        } else {
                            (rejected.clone(), false)
                        };

                        bco_samples.push(BCOSample {
                            prompt: sample.prompt.clone().unwrap_or_default(),
                            completion,
                            label,
                        });
                    }
                }
                TrainingFormat::Completion { .. } => {
                    // Already in correct format
                    if let Some(completion) = &sample.completion {
                        let label = sample.label.map(|l| l > 0.5).unwrap_or(true);
                        bco_samples.push(BCOSample {
                            prompt: sample.prompt.clone().unwrap_or_default(),
                            completion: completion.clone(),
                            label,
                        });
                    }
                }
                _ => {
                    // For other formats, try to extract completion
                    if let Some(completion) = &sample.completion {
                        bco_samples.push(BCOSample {
                            prompt: sample.prompt.clone().unwrap_or_default(),
                            completion: completion.clone(),
                            label: sample.label.map(|l| l > 0.5).unwrap_or(true),
                        });
                    }
                }
            }
        }

        Ok(BCODataset { samples: bco_samples })
    }

    fn to_dpo_format(&self) -> Result<DPODataset> {
        let mut dpo_samples = Vec::new();

        for sample in &self.samples {
            if let TrainingFormat::Preference { .. } = &self.format {
                if let (Some(chosen), Some(rejected)) = (&sample.chosen, &sample.rejected) {
                    dpo_samples.push(DPOSample {
                        prompt: sample.prompt.clone().unwrap_or_default(),
                        chosen: chosen.clone(),
                        rejected: rejected.clone(),
                    });
                }
            } else {
                bail!("DPO format requires preference-style data (chosen/rejected fields)");
            }
        }

        Ok(DPODataset { samples: dpo_samples })
    }

    fn to_ppo_format(&self) -> Result<PPODataset> {
        // PPO typically uses the same format as SFT but with rewards
        let mut ppo_samples = Vec::new();

        for sample in &self.samples {
            if let Some(completion) = &sample.completion {
                ppo_samples.push(PPOSample {
                    prompt: sample.prompt.clone().unwrap_or_default(),
                    completion: completion.clone(),
                    reward: sample.label.unwrap_or(0.0),
                });
            }
        }

        Ok(PPODataset { samples: ppo_samples })
    }

    fn to_sft_format(&self) -> Result<SFTDataset> {
        let mut sft_samples = Vec::new();

        for sample in &self.samples {
            if let Some(completion) = &sample.completion {
                sft_samples.push(SFTSample {
                    prompt: sample.prompt.clone().unwrap_or_default(),
                    completion: completion.clone(),
                });
            }
        }

        Ok(SFTDataset { samples: sft_samples })
    }

    /// Get dataset quality assessment
    pub fn quality_assessment(&self) -> DatasetQualityReport {
        let mut report = DatasetQualityReport {
            overall_score: 0.0,
            issues: Vec::new(),
            recommendations: Vec::new(),
        };

        // Check field coverage
        let prompt_coverage = self.statistics.field_coverage.get("prompt").unwrap_or(&0.0);
        let completion_coverage = self.statistics.field_coverage.get("completion").unwrap_or(&0.0);
        let chosen_coverage = self.statistics.field_coverage.get("chosen").unwrap_or(&0.0);
        let rejected_coverage = self.statistics.field_coverage.get("rejected").unwrap_or(&0.0);

        match &self.format {
            TrainingFormat::Preference { .. } => {
                if *chosen_coverage < 0.9 {
                    report.issues.push(format!("Low chosen field coverage: {:.1}%", chosen_coverage * 100.0));
                }
                if *rejected_coverage < 0.9 {
                    report.issues.push(format!("Low rejected field coverage: {:.1}%", rejected_coverage * 100.0));
                }
                report.overall_score = (chosen_coverage + rejected_coverage + prompt_coverage) / 3.0;
            }
            TrainingFormat::Completion { .. } => {
                if *completion_coverage < 0.9 {
                    report.issues.push(format!("Low completion field coverage: {:.1}%", completion_coverage * 100.0));
                }
                report.overall_score = (completion_coverage + prompt_coverage) / 2.0;
            }
            _ => {
                report.overall_score = self.statistics.quality_score.unwrap_or(0.0);
            }
        }

        // Check content quality
        if self.statistics.avg_prompt_length < 10.0 {
            report.issues.push("Very short average prompt length".to_string());
        }
        if self.statistics.avg_completion_length < 10.0 {
            report.issues.push("Very short average completion length".to_string());
        }

        // Generate recommendations
        if report.issues.is_empty() {
            report.recommendations.push("Dataset quality looks good!".to_string());
        } else {
            report.recommendations.push("Consider filtering or augmenting low-quality samples".to_string());
        }

        report
    }
}

/// Algorithm-specific dataset structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BCOSample {
    pub prompt: String,
    pub completion: String,
    pub label: bool,
}

#[derive(Debug, Clone)]
pub struct BCODataset {
    pub samples: Vec<BCOSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DPOSample {
    pub prompt: String,
    pub chosen: String,
    pub rejected: String,
}

#[derive(Debug, Clone)]
pub struct DPODataset {
    pub samples: Vec<DPOSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PPOSample {
    pub prompt: String,
    pub completion: String,
    pub reward: f32,
}

#[derive(Debug, Clone)]
pub struct PPODataset {
    pub samples: Vec<PPOSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SFTSample {
    pub prompt: String,
    pub completion: String,
}

#[derive(Debug, Clone)]
pub struct SFTDataset {
    pub samples: Vec<SFTSample>,
}

/// Dataset quality assessment report
#[derive(Debug, Clone)]
pub struct DatasetQualityReport {
    pub overall_score: f64,
    pub issues: Vec<String>,
    pub recommendations: Vec<String>,
}

fn clean_whitespace(text: &str) -> String {
    text.lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}
fn process_file(
    src_path: &Path,
    dst_path: &Path,
    schema_path: Option<&Path>,
    format_override: &DataFormat,
) -> Result<()> {
    let content = fs::read_to_string(src_path)
        .with_context(|| format!("Failed to read {}", src_path.display()))?;
    let raw: Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse JSON in {}", src_path.display()))?;
    let _temp_dataset = GenericJSONDataset::new(
        &[src_path.to_path_buf()],
        schema_path,
        format_override.clone(),
    )?;
    let cleaned = if let Value::Array(arr) = raw {
        let cleaned_arr: Vec<Value> = arr
            .into_iter()
            .map(|mut entry| {
                if let Value::Object(ref mut obj) = entry {
                    for (_key, value) in obj.iter_mut() {
                        if let Value::String(ref mut s) = value {
                            *s = clean_whitespace(s);
                        }
                    }
                }
                entry
            })
            .collect();
        Value::Array(cleaned_arr)
    } else {
        raw
    };
    dst_path
        .parent()
        .map(|p| fs::create_dir_all(p))
        .transpose()
        .with_context(|| {
            format!("Failed to create directory for {}", dst_path.display())
        })?;
    let cleaned_json = serde_json::to_string_pretty(&cleaned)
        .with_context(|| "Failed to serialize cleaned JSON")?;
    fs::write(dst_path, cleaned_json)
        .with_context(|| format!("Failed to write to {}", dst_path.display()))?;
    Ok(())
}
pub async fn run_multi_process_clean(
    src_files: Vec<PathBuf>,
    dst_root: &Path,
    schema_dir: Option<&Path>,
    format_override: &DataFormat,
    _jobs: usize,
) -> Result<()> {
    if src_files.is_empty() {
        bail!("No source files provided");
    }
    let tasks: Vec<_> = src_files
        .iter()
        .map(|src| {
            let dst = dst_root.join(src.file_name().unwrap());
            let schema_path = schema_dir
                .and_then(|dir| {
                    let candidate = dir
                        .join(format!("{}.schema.json", src.file_stem() ?.to_str() ?));
                    if candidate.is_file() { Some(candidate) } else { None }
                });
            (src.clone(), dst, schema_path)
        })
        .collect();
    let pb = ProgressBar::new(tasks.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta}) {msg}",
            )
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message("Cleaning & validating");
    let pb = Arc::new(Mutex::new(pb));
    let results: Vec<Result<()>> = tasks
        .iter()
        .map(|(src, dst, schema_path)| {
            let result = process_file(
                src,
                dst,
                schema_path.as_deref(),
                format_override,
            );
            if let Ok(pb) = pb.try_lock() {
                pb.inc(1);
            }
            result
        })
        .collect();
    if let Ok(pb) = pb.try_lock() {
        pb.finish_with_message("Complete");
    }
    let errors: Vec<_> = results
        .into_iter()
        .enumerate()
        .filter_map(|(i, r)| r.err().map(|e| (i, e)))
        .collect();
    if !errors.is_empty() {
        for (i, e) in &errors {
            eprintln!("Error processing file {}: {}", i, e);
        }
        bail!("{} files failed processing", errors.len());
    }
    Ok(())
}
pub fn gather_json_paths(
    files: &[String],
    data_dirs: &[String],
) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for file in files {
        let path = PathBuf::from(file);
        if !path.is_file() {
            bail!("Specified file does not exist: {}", path.display());
        }
        paths.push(path);
    }
    for dir in data_dirs {
        let dir_path = PathBuf::from(dir);
        if !dir_path.is_dir() {
            bail!("Data directory does not exist: {}", dir_path.display());
        }
        for entry in WalkDir::new(&dir_path).into_iter().filter_map(|e| e.ok()) {
            if entry.path().extension().map_or(false, |ext| ext == "json") {
                paths.push(entry.path().to_path_buf());
            }
        }
    }
    if paths.is_empty() {
        bail!("No JSON files found in specified paths");
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}
pub fn find_schema_for_file(
    json_path: &Path,
    schema_dir: Option<&Path>,
) -> Option<PathBuf> {
    schema_dir
        .and_then(|dir| {
            let candidate = dir
                .join(format!("{}.schema.json", json_path.file_stem() ?.to_str() ?));
            if candidate.is_file() { Some(candidate) } else { None }
        })
}
pub async fn run_json_cmd(args: JsonArgs) -> Result<()> {
    if args.multi_process {
        if args.input_folder.is_none() || args.output.is_none() {
            bail!("Multi-process mode requires both --input-folder and --output");
        }
        let src_root = args.input_folder.unwrap();
        let dst_root = args.output.unwrap();
        if !src_root.is_dir() {
            bail!("Input folder does not exist: {}", src_root.display());
        }
        let src_files: Vec<_> = WalkDir::new(&src_root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "json"))
            .map(|e| e.path().to_path_buf())
            .collect();
        if src_files.is_empty() {
            bail!("No JSON files found in {}", src_root.display());
        }
        let schema_dir = args.schema_dir.as_ref();
        let format_override = args.format.clone();
        println!(
            "🔧 Starting multi-process clean-validate:\n  source: {}\n  destination: {}\n  workers: {}\n  format: {:?}",
            src_root.display(), dst_root.display(), args.jobs, format_override
        );
        run_multi_process_clean(
                src_files,
                &dst_root,
                schema_dir.map(|p| p.as_path()),
                &format_override,
                args.jobs,
            )
            .await?;
        println!("✅ All files cleaned and validated successfully.");
        return Ok(());
    }
    let json_paths = gather_json_paths(&args.file, &args.data_dir)?;
    let mut all_samples = Vec::new();
    for path in &json_paths {
        let schema_path = find_schema_for_file(
            path,
            args.schema_dir.as_ref().map(|p| p.as_path()),
        );
        println!(
            "Loading {} (schema: {})", path.file_name().unwrap().to_string_lossy(),
            schema_path.as_ref().map(| p | p.file_name().unwrap().to_string_lossy())
            .unwrap_or(std::borrow::Cow::Borrowed("built-in"))
        );
        let dataset = GenericJSONDataset::new(
            &[path.clone()],
            schema_path.as_ref().map(|p| p.as_path()),
            args.format.clone(),
        )?;
        all_samples.extend(dataset.data);
    }
    if let Some(merge_output) = &args.merge_output {
        merge_output
            .parent()
            .map(|p| fs::create_dir_all(p))
            .transpose()
            .with_context(|| {
                format!("Failed to create directory for {}", merge_output.display())
            })?;
        let merged_json = serde_json::to_string_pretty(&all_samples)
            .with_context(|| "Failed to serialize merged JSON")?;
        fs::write(merge_output, merged_json)
            .with_context(|| {
                format!("Failed to write merged output to {}", merge_output.display())
            })?;
        println!(
            "✅ Merged {} samples into {}", all_samples.len(), merge_output.display()
        );
    }
    if args.show_stats {
        if !json_paths.is_empty() {
            let temp_dataset = GenericJSONDataset::new(
                &json_paths,
                args.schema_dir.as_ref().map(|p| p.as_path()),
                args.format,
            )?;
            println!(
                "\n--- Dataset statistics ------------------------------------------------"
            );
            for (k, v) in temp_dataset.stats() {
                println!("{:20}: {}", k, v);
            }
            println!(
                "--------------------------------------------------------------------"
            );
        }
    }
    if args.merge_output.is_none() {
        println!("🎉 Validation finished – no merged output requested.");
    }
    Ok(())
}
#[derive(Args, Debug, Clone)]
pub struct JsonArgs {
    #[arg(long)]
    pub data_dir: Vec<String>,
    #[arg(long, short = 'f')]
    pub file: Vec<String>,
    #[arg(long)]
    pub schema_dir: Option<PathBuf>,
    #[arg(long, default_value = "auto")]
    pub format: DataFormat,
    #[arg(long)]
    pub merge_output: Option<PathBuf>,
    #[arg(long)]
    pub show_stats: bool,
    #[arg(long, default_value = "42")]
    pub seed: u64,
    #[arg(long)]
    pub multi_process: bool,
    #[arg(long)]
    pub input_folder: Option<PathBuf>,
    #[arg(long, short = 'o')]
    pub output: Option<PathBuf>,
    #[arg(long, short = 'j', default_value_t = num_cpus::get())]
    pub jobs: usize,
}