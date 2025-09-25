use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use crate::error::HlxError;
use crate::value::Value;
use arrow::datatypes::{Schema, Field, DataType};
use arrow::array::{Array, ArrayRef, StringArray, Float64Array, Int64Array};
use arrow::record_batch::RecordBatch;
use std::sync::Arc;

// Data and config format support
pub mod helix_format;        // .helix data files
pub mod hlxc_format;       // .hlxc compressed data files
pub mod hlx_config_format; // .hlx text config files
pub mod hlxb_config_format; // .hlxb binary config files

use hlxc_format::HlxcDataWriter;

/// Supported output formats
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum OutputFormat {
    /// HLX binary format with Arrow IPC (.helix files)
    Helix,
    /// HLXC compressed binary format with custom header (.hlxc files)
    Hlxc,
    /// Parquet columnar format
    Parquet,
    /// MessagePack binary format
    MsgPack,
    /// JSON Lines (fallback for debugging)
    Jsonl,
    /// CSV format (fallback)
    Csv,
}

impl OutputFormat {
    /// Parse an output format from a string
    pub fn from(s: &str) -> Result<Self, HlxError> {
        match s.to_lowercase().as_str() {
            "helix" | "hlx" => Ok(OutputFormat::Helix),
            "hlxc" | "compressed" => Ok(OutputFormat::Hlxc),
            "parquet" => Ok(OutputFormat::Parquet),
            "msgpack" | "messagepack" => Ok(OutputFormat::MsgPack),
            "jsonl" | "json" => Ok(OutputFormat::Jsonl),
            "csv" => Ok(OutputFormat::Csv),
            _ => Err(HlxError::validation_error(
                format!("Unsupported output format: {}", s),
                "Supported formats: helix, hlxc, parquet, msgpack, jsonl, csv"
            )),
        }
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = HlxError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from(s)
    }
}

/// Output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Output directory
    pub output_dir: PathBuf,
    /// Supported formats (first is primary)
    pub formats: Vec<OutputFormat>,
    /// Compression settings
    pub compression: CompressionConfig,
    /// Batch size for processing
    pub batch_size: usize,
    /// Whether to include preview rows
    pub include_preview: bool,
    /// Maximum preview rows
    pub preview_rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    pub enabled: bool,
    pub algorithm: CompressionAlgorithm,
    pub level: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    Zstd,
    Lz4,
    Snappy,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            algorithm: CompressionAlgorithm::Zstd,
            level: 4,
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("output"),
            formats: vec![OutputFormat::Helix, OutputFormat::Jsonl],
            compression: CompressionConfig::default(),
            batch_size: 1000,
            include_preview: true,
            preview_rows: 10,
        }
    }
}

/// Data output writer trait
pub trait DataWriter {
    fn write_batch(&mut self, batch: RecordBatch) -> Result<(), HlxError>;
    fn finalize(&mut self) -> Result<(), HlxError>;
}

/// Main output manager
pub struct OutputManager {
    config: OutputConfig,
    writers: HashMap<OutputFormat, Box<dyn DataWriter>>,
    current_batch: Vec<HashMap<String, Value>>,
    schema: Option<Schema>,
    batch_count: usize,
    writers_initialized: bool,
}

impl OutputManager {
    pub fn new(config: OutputConfig) -> Self {
        Self {
            config,
            writers: HashMap::new(),
            current_batch: Vec::new(),
            schema: None,
            batch_count: 0,
            writers_initialized: false,
        }
    }

    pub fn add_row(&mut self, row: HashMap<String, Value>) -> Result<(), HlxError> {
        // Infer schema from first row if not set
        if self.schema.is_none() {
            self.schema = Some(infer_schema(&row));
        }

        self.current_batch.push(row);

        // Write batch if it reaches the configured size
        if self.current_batch.len() >= self.config.batch_size {
            self.flush_batch()?;
        }

        Ok(())
    }

    pub fn flush_batch(&mut self) -> Result<(), HlxError> {
        if self.current_batch.is_empty() {
            return Ok(());
        }

        if let Some(schema) = &self.schema {
            let batch = convert_to_record_batch(schema, &self.current_batch)?;
            self.write_batch_to_all_writers(batch)?;
        }

        self.current_batch.clear();
        Ok(())
    }

    pub fn finalize_all(&mut self) -> Result<(), HlxError> {
        self.flush_batch()?;
        for writer in self.writers.values_mut() {
            writer.finalize()?;
        }
        Ok(())
    }

    /// Initialize writers for all configured formats
    fn initialize_writers(&mut self) -> Result<(), HlxError> {
        if self.writers_initialized {
            return Ok(());
        }

        for format in &self.config.formats {
            let writer: Box<dyn DataWriter> = match format {
                OutputFormat::Hlxc => {
                    Box::new(HlxcDataWriter::new(self.config.clone()))
                }
                // TODO: Add other format implementations
                _ => {
                    // For now, skip unsupported formats
                    continue;
                }
            };
            self.writers.insert(format.clone(), writer);
        }

        self.writers_initialized = true;
        Ok(())
    }

    fn write_batch_to_all_writers(&mut self, batch: RecordBatch) -> Result<(), HlxError> {
        // Initialize writers if not done yet
        self.initialize_writers()?;

        // Write to all initialized writers
        for (format, writer) in &mut self.writers {
            if *format == OutputFormat::Hlxc {
                writer.write_batch(batch.clone())?;
            }
            // TODO: Add support for other formats
        }

        Ok(())
    }

    pub fn get_output_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();

        for format in &self.config.formats {
            let extension = match format {
                OutputFormat::Helix => "helix",
                OutputFormat::Hlxc => "hlxc",
                OutputFormat::Parquet => "parquet",
                OutputFormat::MsgPack => "msgpack",
                OutputFormat::Jsonl => "jsonl",
                OutputFormat::Csv => "csv",
            };

            let filename = format!("output_{:04}.{}", self.batch_count, extension);
            files.push(self.config.output_dir.join(filename));
        }

        files
    }
}

/// Infer Arrow schema from a sample row
fn infer_schema(row: &HashMap<String, Value>) -> Schema {
    let fields: Vec<arrow::datatypes::Field> = row.iter().map(|(name, value)| {
        let data_type = match value {
            Value::String(_) => DataType::Utf8,
            Value::Number(_) => DataType::Float64,
            Value::Bool(_) => DataType::Boolean,
            _ => DataType::Utf8, // Default to string for other types
        };
        Field::new(name, data_type, true)
    }).collect();

    Schema::new(fields)
}

/// Convert a batch of hashmaps to Arrow RecordBatch
fn convert_to_record_batch(schema: &Schema, batch: &[HashMap<String, Value>]) -> Result<RecordBatch, HlxError> {
    let arrays: Result<Vec<ArrayRef>, HlxError> = schema.fields().iter().map(|field| {
        let column_data: Vec<Value> = batch.iter().map(|row| {
            row.get(field.name()).cloned().unwrap_or(Value::Null)
        }).collect();

        match field.data_type() {
            DataType::Utf8 => {
                let string_data: Vec<Option<String>> = column_data.into_iter().map(|v| {
                    match v {
                        Value::String(s) => Some(s),
                        _ => Some(v.to_string()),
                    }
                }).collect();
                Ok(Arc::new(StringArray::from(string_data)) as ArrayRef)
            }
            DataType::Float64 => {
                let float_data: Vec<Option<f64>> = column_data.into_iter().map(|v| {
                    match v {
                        Value::Number(n) => Some(n),
                        Value::String(s) => s.parse().ok(),
                        _ => None,
                    }
                }).collect();
                Ok(Arc::new(Float64Array::from(float_data)) as ArrayRef)
            }
            DataType::Int64 => {
                let int_data: Vec<Option<i64>> = column_data.into_iter().map(|v| {
                    match v {
                        Value::Number(n) => Some(n as i64),
                        Value::String(s) => s.parse().ok(),
                        _ => None,
                    }
                }).collect();
                Ok(Arc::new(Int64Array::from(int_data)) as ArrayRef)
            }
            DataType::Boolean => {
                let bool_data: Vec<Option<bool>> = column_data.into_iter().map(|v| {
                    match v {
                        Value::Bool(b) => Some(b),
                        Value::String(s) => match s.to_lowercase().as_str() {
                            "true" | "1" | "yes" => Some(true),
                            "false" | "0" | "no" => Some(false),
                            _ => None,
                        },
                        _ => None,
                    }
                }).collect();
                Ok(Arc::new(arrow::array::BooleanArray::from(bool_data)) as ArrayRef)
            }
            _ => {
                // Default to string array
                let string_data: Vec<Option<String>> = column_data.into_iter().map(|v| {
                    Some(v.to_string())
                }).collect();
                Ok(Arc::new(StringArray::from(string_data)) as ArrayRef)
            }
        }
    }).collect();

    let arrays = arrays?;
    RecordBatch::try_new(Arc::new(schema.clone()), arrays)
        .map_err(|e| HlxError::validation_error(format!("Failed to create record batch: {}", e), ""))
}

/// Convert RecordBatch back to hashmap for compatibility
fn convert_batch_to_hashmap(batch: &RecordBatch) -> HashMap<String, Value> {
    let mut result = HashMap::new();

    for (field_idx, field) in batch.schema().fields().iter().enumerate() {
        if let Some(array) = batch.column(field_idx).as_any().downcast_ref::<StringArray>() {
            let values: Vec<Value> = (0..batch.num_rows())
                .map(|i| {
                    if array.is_valid(i) {
                        Value::String(array.value(i).to_string())
                    } else {
                        Value::Null
                    }
                })
                .collect();
            result.insert(field.name().clone(), Value::Array(values));
        } else if let Some(array) = batch.column(field_idx).as_any().downcast_ref::<Float64Array>() {
            let values: Vec<Value> = (0..batch.num_rows())
                .map(|i| {
                    if array.is_valid(i) {
                        Value::Number(array.value(i))
                    } else {
                        Value::Null
                    }
                })
                .collect();
            result.insert(field.name().clone(), Value::Array(values));
        } else if let Some(array) = batch.column(field_idx).as_any().downcast_ref::<Int64Array>() {
            let values: Vec<Value> = (0..batch.num_rows())
                .map(|i| {
                    if array.is_valid(i) {
                        Value::Number(array.value(i) as f64)
                    } else {
                        Value::Null
                    }
                })
                .collect();
            result.insert(field.name().clone(), Value::Array(values));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_infer_schema() {
        let mut row = HashMap::new();
        row.insert("name".to_string(), Value::String("John".to_string()));
        row.insert("age".to_string(), Value::Number(30.0));
        row.insert("active".to_string(), Value::Bool(true));

        let schema = infer_schema(&row);
        assert_eq!(schema.fields().len(), 3);

        assert_eq!(schema.field(0).name(), "name");
        assert_eq!(schema.field(0).data_type(), &DataType::Utf8);

        assert_eq!(schema.field(1).name(), "age");
        assert_eq!(schema.field(1).data_type(), &DataType::Float64);
    }

    #[test]
    fn test_output_format_from_str() {
        assert_eq!(OutputFormat::from("helix").expect("Failed to parse 'helix'"), OutputFormat::Helix);
        assert_eq!(OutputFormat::from("hlxc").expect("Failed to parse 'hlxc'"), OutputFormat::Hlxc);
        assert_eq!(OutputFormat::from("compressed").expect("Failed to parse 'compressed'"), OutputFormat::Hlxc);
        assert_eq!(OutputFormat::from("parquet").expect("Failed to parse 'parquet'"), OutputFormat::Parquet);
        assert_eq!(OutputFormat::from("msgpack").expect("Failed to parse 'msgpack'"), OutputFormat::MsgPack);
        assert_eq!(OutputFormat::from("jsonl").expect("Failed to parse 'jsonl'"), OutputFormat::Jsonl);
        assert_eq!(OutputFormat::from("csv").expect("Failed to parse 'csv'"), OutputFormat::Csv);

        assert!(OutputFormat::from("invalid").is_err());
    }
}
