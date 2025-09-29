use std::path::{Path, PathBuf};
use std::collections::HashMap;
use crate::hel::error::{HlxError, HlxResult};
use crate::{HelixConfig, parse, validate, ast_to_config};
use crate::dna::compiler::{Compiler, OptimizationLevel};
use crate::mds::loader::BinaryLoader; 
pub struct HlxDatasetProcessor {
    #[cfg(feature = "compiler")]
    compiler: Compiler,
    cache_dir: PathBuf,
    config_cache: HashMap<PathBuf, HelixConfig>,
}
impl Default for HlxDatasetProcessor {
    fn default() -> Self {
        Self::new()
    }
}
impl HlxDatasetProcessor {
    #[cfg(feature = "compiler")]
    pub fn new() -> Self {
        Self {
            compiler: Compiler::new(OptimizationLevel::Two),
            cache_dir: PathBuf::from("./hlx_cache"),
            config_cache: HashMap::new(),
        }
    }
    #[cfg(not(feature = "compiler"))]
    pub fn new() -> Self {
        Self {
            cache_dir: PathBuf::from("./hlx_cache"),
            config_cache: HashMap::new(),
        }
    }
    pub fn load_config_file<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> HlxResult<HelixConfig> {
        let path = path.as_ref().to_path_buf();
        if let Some(config) = self.config_cache.get(&path) {
            return Ok(config.clone());
        }
        if !path.exists() {
            return Err(HlxError::dataset_not_found(path));
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| HlxError::DatasetProcessing {
                message: format!("Failed to read HLX file: {}", e),
                suggestion: "Ensure the file exists and is readable".to_string(),
            })?;
        let config = self.parse_hlx_content(&content)?;
        self.config_cache.insert(path, config.clone());
        Ok(config)
    }
    pub fn parse_hlx_content(&self, content: &str) -> HlxResult<HelixConfig> {
        let ast = parse(content)
            .map_err(|e| HlxError::HlxProcessing {
                message: format!("HLX parsing failed: {}", e),
                suggestion: "Check HLX syntax and ensure all required fields are present"
                    .to_string(),
            })?;
        validate(&ast)
            .map_err(|e| HlxError::ConfigValidation {
                field: "validation".to_string(),
                value: "failed".to_string(),
                suggestion: format!("HLX validation failed: {}", e),
            })?;
        let config = ast_to_config(ast)
            .map_err(|e| HlxError::ConfigConversion {
                field: "ast_to_config".to_string(),
                details: e.to_string(),
                suggestion: "Check HLX configuration structure and required fields"
                    .to_string(),
            })?;
        Ok(config)
    }
    #[cfg(feature = "compiler")]
    pub fn load_binary_file<P: AsRef<Path>>(&self, path: P) -> HlxResult<HelixConfig> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(HlxError::dataset_not_found(path.to_path_buf()));
        }
        let loader = BinaryLoader::new();
        let binary = loader
            .load_file(path)
            .map_err(|e| HlxError::HlxProcessing {
                message: format!("Failed to load HLX binary: {}", e),
                suggestion: "Ensure the binary file is valid and not corrupted"
                    .to_string(),
            })?;
        let source = self
            .compiler
            .decompile(&binary)
            .map_err(|e| HlxError::HlxProcessing {
                message: format!("Failed to decompile HLX binary: {}", e),
                suggestion: "The binary may be corrupted or from an incompatible version"
                    .to_string(),
            })?;
        let ast = crate::parse(&source)
            .map_err(|e| HlxError::HlxProcessing {
                message: format!("Failed to parse decompiled source: {}", e),
                suggestion: "The decompiled source may be corrupted".to_string(),
            })?;
        let config = ast_to_config(ast)
            .map_err(|e| HlxError::ConfigConversion {
                field: "binary_decompile".to_string(),
                details: e.to_string(),
                suggestion: "Check if the binary was compiled with a compatible HLX version"
                    .to_string(),
            })?;
        Ok(config)
    }
    #[cfg(feature = "compiler")]
    pub fn compile_to_binary(
        &self,
        content: &str,
    ) -> HlxResult<crate::dna::hel::binary::HelixBinary> {
        let ast = parse(content)
            .map_err(|e| HlxError::HlxProcessing {
                message: format!("HLX parsing failed: {}", e),
                suggestion: "Check HLX syntax before compilation".to_string(),
            })?;
        validate(&ast)
            .map_err(|_e| HlxError::ConfigValidation {
                field: "pre_compile_validation".to_string(),
                value: "failed".to_string(),
                suggestion: "Run validation separately before compilation".to_string(),
            })?;
        let source = crate::pretty_print(&ast);
        let binary = self
            .compiler
            .compile_source(&source, None)
            .map_err(|e| HlxError::HlxProcessing {
                message: format!("HLX compilation failed: {}", e),
                suggestion: "Check for semantic errors in the HLX configuration"
                    .to_string(),
            })?;
        Ok(binary)
    }
    pub fn process_dataset_config(
        &mut self,
        config_path: &str,
        dataset_name: &str,
    ) -> HlxResult<DatasetConfig> {
        let config = self.load_config_file(config_path)?;
        let dataset_config = self.extract_dataset_config(&config, dataset_name)?;
        Ok(dataset_config)
    }
    fn extract_dataset_config(
        &self,
        _config: &HelixConfig,
        dataset_name: &str,
    ) -> HlxResult<DatasetConfig> {
        let dataset_config = DatasetConfig {
            name: dataset_name.to_string(),
            source: format!("hlx://{}", dataset_name),
            format: "auto".to_string(),
            validation_rules: vec![
                "check_required_fields".to_string(), "validate_data_types".to_string(),
            ],
            processing_options: ProcessingOptions {
                batch_size: 32,
                shuffle: true,
                filter_duplicates: true,
            },
        };
        Ok(dataset_config)
    }
    pub fn validate_dataset(
        &self,
        dataset_config: &DatasetConfig,
        data_sample: &serde_json::Value,
    ) -> HlxResult<ValidationResult> {
        let mut issues = Vec::new();
        let mut score: f64 = 1.0;
        if dataset_config.format == "auto" {
            if let Some(prompt) = data_sample.get("prompt") {
                if prompt.as_str().map(|s| s.len()).unwrap_or(0) < 10 {
                    issues.push("Prompt too short (< 10 chars)".to_string());
                    score -= 0.2;
                }
            } else {
                issues.push("Missing required 'prompt' field".to_string());
                score -= 0.5;
            }
        }
        let suggestions = if issues.is_empty() {
            vec!["Dataset validation passed".to_string()]
        } else {
            vec![
                "Review dataset format and required fields".to_string(),
                "Check data quality and completeness".to_string(),
            ]
        };
        Ok(ValidationResult {
            is_valid: issues.is_empty(),
            score: score.max(0.0),
            issues,
            suggestions,
        })
    }
    pub fn cache_stats(&self) -> std::io::Result<CacheStats> {
        if !self.cache_dir.exists() {
            return Ok(CacheStats::default());
        }
        let mut total_size = 0u64;
        let mut file_count = 0u32;
        if let Ok(entries) = std::fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        total_size += metadata.len();
                        file_count += 1;
                    }
                }
            }
        }
        Ok(CacheStats {
            total_size_bytes: total_size,
            file_count,
            cache_dir: self.cache_dir.clone(),
            cached_configs: self.config_cache.len() as u32,
        })
    }
    pub fn clear_cache(&mut self) -> std::io::Result<()> {
        self.config_cache.clear();
        if self.cache_dir.exists() {
            std::fs::remove_dir_all(&self.cache_dir)?;
        }
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct DatasetConfig {
    pub name: String,
    pub source: String,
    pub format: String,
    pub validation_rules: Vec<String>,
    pub processing_options: ProcessingOptions,
}
#[derive(Debug, Clone)]
pub struct ProcessingOptions {
    pub batch_size: usize,
    pub shuffle: bool,
    pub filter_duplicates: bool,
}
impl Default for ProcessingOptions {
    fn default() -> Self {
        Self {
            batch_size: 32,
            shuffle: true,
            filter_duplicates: true,
        }
    }
}
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub score: f64,
    pub issues: Vec<String>,
    pub suggestions: Vec<String>,
}
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_size_bytes: u64,
    pub file_count: u32,
    pub cache_dir: PathBuf,
    pub cached_configs: u32,
}
impl Default for CacheStats {
    fn default() -> Self {
        Self {
            total_size_bytes: 0,
            file_count: 0,
            cache_dir: PathBuf::from("./hlx_cache"),
            cached_configs: 0,
        }
    }
}
impl CacheStats {
    pub fn total_size_mb(&self) -> f64 {
        self.total_size_bytes as f64 / (1024.0 * 1024.0)
    }
    pub fn total_size_gb(&self) -> f64 {
        self.total_size_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }
}
pub struct HlxBridge {
    processor: HlxDatasetProcessor,
}
impl Default for HlxBridge {
    fn default() -> Self {
        Self::new()
    }
}
impl HlxBridge {
    pub fn new() -> Self {
        Self {
            processor: HlxDatasetProcessor::new(),
        }
    }
    pub fn convert_legacy_dataset(
        &mut self,
        legacy_config: &std::collections::HashMap<String, String>,
    ) -> HlxResult<DatasetConfig> {
        let name = legacy_config
            .get("dataset.name")
            .cloned()
            .unwrap_or_else(|| "converted_dataset".to_string());
        let format = legacy_config
            .get("dataset.format")
            .cloned()
            .unwrap_or_else(|| "auto".to_string());
        let batch_size = legacy_config
            .get("processing.batch_size")
            .and_then(|s| s.parse().ok())
            .unwrap_or(32) as usize;
        let shuffle = legacy_config
            .get("processing.shuffle")
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);
        Ok(DatasetConfig {
            name: name.clone(),
            source: format!("legacy://{}", name),
            format,
            validation_rules: vec![
                "legacy_compatibility_check".to_string(), "format_validation"
                .to_string(),
            ],
            processing_options: ProcessingOptions {
                batch_size,
                shuffle,
                filter_duplicates: true,
            },
        })
    }
    pub async fn process_with_hlx(
        &mut self,
        hlx_config_path: &str,
        dataset_data: Vec<serde_json::Value>,
    ) -> HlxResult<ProcessedDataset> {
        let _config = self.processor.load_config_file(hlx_config_path)?;
        let mut processed_samples = Vec::new();
        let mut total_score = 0.0;
        let data_len = dataset_data.len();
        for sample in &dataset_data {
            let dataset_config = DatasetConfig {
                name: "processing".to_string(),
                source: "inline".to_string(),
                format: "auto".to_string(),
                validation_rules: vec![],
                processing_options: ProcessingOptions {
                    batch_size: 1,
                    shuffle: false,
                    filter_duplicates: false,
                },
            };
            let validation = self.processor.validate_dataset(&dataset_config, sample)?;
            total_score += validation.score;
            if validation.is_valid {
                processed_samples.push(sample.clone());
            }
        }
        let avg_score = if data_len > 0 { total_score / data_len as f64 } else { 0.0 };
        let valid_count = processed_samples.len();
        Ok(ProcessedDataset {
            samples: processed_samples,
            quality_score: avg_score,
            total_processed: data_len,
            valid_samples: valid_count,
        })
    }
}
#[derive(Debug, Clone)]
pub struct ProcessedDataset {
    pub samples: Vec<serde_json::Value>,
    pub quality_score: f64,
    pub total_processed: usize,
    pub valid_samples: usize,
}
pub fn start_server(config: crate::mds::server::ServerConfig) -> crate::mds::server::HelixServer {
    crate::mds::server::HelixServer::new(config)
}
pub fn start_default_server() -> crate::mds::server::HelixServer {
    crate::mds::server::HelixServer::new(crate::mds::server::ServerConfig::default())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_processor_creation() {
        let processor = HlxDatasetProcessor::new();
        assert!(processor.cache_dir.ends_with("hlx_cache"));
    }
    #[test]
    fn test_cache_stats() {
        let processor = HlxDatasetProcessor::new();
        let stats = processor.cache_stats().unwrap();
        assert_eq!(stats.file_count, 0);
        assert_eq!(stats.cached_configs, 0);
    }
    #[test]
    fn test_validation_result() {
        let result = ValidationResult {
            is_valid: true,
            score: 1.0,
            issues: vec![],
            suggestions: vec!["All good".to_string()],
        };
        assert!(result.is_valid);
        assert_eq!(result.score, 1.0);
    }
    #[test]
    fn test_dataset_config() {
        let config = DatasetConfig {
            name: "test".to_string(),
            source: "file://test.json".to_string(),
            format: "json".to_string(),
            validation_rules: vec!["required_fields".to_string()],
            processing_options: ProcessingOptions {
                batch_size: 32,
                shuffle: true,
                filter_duplicates: true,
            },
        };
        assert_eq!(config.name, "test");
        assert_eq!(config.processing_options.batch_size, 32);
    }
    #[tokio::test]
    async fn test_bridge_processing() {
        let mut bridge = HlxBridge::new();
        let result = bridge.process_with_hlx("dummy.hlx", vec![]).await;
        assert!(result.is_ok());
        let processed = result.unwrap();
        assert_eq!(processed.samples.len(), 0);
        assert_eq!(processed.quality_score, 0.0);
    }
}