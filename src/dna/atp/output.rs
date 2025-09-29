use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use crate::hel::error::HlxError;
use crate::atp::types::Value;
use arrow::datatypes::{Schema, Field, DataType};
use arrow::array::{Array, ArrayRef, StringArray, Float64Array, Int64Array};
use arrow::record_batch::RecordBatch;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum OutputFormat {
    Helix,
    Hlxc,
    Parquet,
    MsgPack,
    Jsonl,
    Csv,
}
impl OutputFormat {
    pub fn from(s: &str) -> Result<Self, HlxError> {
        match s.to_lowercase().as_str() {
            "helix" | "hlx" => Ok(OutputFormat::Helix),
            "hlxc" | "compressed" => Ok(OutputFormat::Hlxc),
            "parquet" => Ok(OutputFormat::Parquet),
            "msgpack" | "messagepack" => Ok(OutputFormat::MsgPack),
            "jsonl" | "json" => Ok(OutputFormat::Jsonl),
            "csv" => Ok(OutputFormat::Csv),
            _ => {
                Err(
                    HlxError::validation_error(
                        format!("Unsupported output format: {}", s),
                        "Supported formats: helix, hlxc, parquet, msgpack, jsonl, csv",
                    ),
                )
            }
        }
    }
}
impl std::str::FromStr for OutputFormat {
    type Err = HlxError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from(s)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub output_dir: PathBuf,
    pub formats: Vec<OutputFormat>,
    pub compression: CompressionConfig,
    pub batch_size: usize,
    pub include_preview: bool,
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
pub trait DataWriter {
    fn write_batch(&mut self, batch: RecordBatch) -> Result<(), HlxError>;
    fn finalize(&mut self) -> Result<(), HlxError>;
}
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
        if self.schema.is_none() {
            self.schema = Some(infer_schema(&row));
        }
        self.current_batch.push(row);
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
    fn initialize_writers(&mut self) -> Result<(), HlxError> {
        if self.writers_initialized {
            return Ok(());
        }
        for format in &self.config.formats {
            let writer: Box<dyn DataWriter> = match format {
                OutputFormat::Hlxc => Box::new(HlxcDataWriter::new(self.config.clone())),
                _ => {
                    continue;
                }
            };
            self.writers.insert(format.clone(), writer);
        }
        self.writers_initialized = true;
        Ok(())
    }
    fn write_batch_to_all_writers(
        &mut self,
        batch: RecordBatch,
    ) -> Result<(), HlxError> {
        self.initialize_writers()?;
        for (format, writer) in &mut self.writers {
            if *format == OutputFormat::Hlxc {
                writer.write_batch(batch.clone())?;
            }
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

pub struct HlxcDataWriter {
    config: OutputConfig,
    buffer: Vec<u8>,
}

impl HlxcDataWriter {
    pub fn new(config: OutputConfig) -> Self {
        Self {
            config,
            buffer: Vec::new(),
        }
    }
}

impl DataWriter for HlxcDataWriter {
    fn write_batch(&mut self, batch: RecordBatch) -> Result<(), HlxError> {
        // For now, just serialize to JSON as placeholder
        // In real implementation, this would write compressed binary format
        let schema_info = format!("{{\"fields\": {}, \"rows\": {}}}", 
            batch.schema().fields().len(), 
            batch.num_rows()
        );
        let data_json = format!("{{\"schema\": {}, \"rows\": {}}}", schema_info, batch.num_rows());
        self.buffer.extend_from_slice(data_json.as_bytes());
        Ok(())
    }

    fn finalize(&mut self) -> Result<(), HlxError> {
        // Write buffer to file if configured
        // For now, this is a placeholder implementation
        Ok(())
    }
}

fn infer_schema(row: &HashMap<String, Value>) -> Schema {
    let fields: Vec<arrow::datatypes::Field> = row
        .iter()
        .map(|(name, value)| {
            let data_type = match value {
                Value::String(_) => DataType::Utf8,
                Value::Number(_) => DataType::Float64,
                Value::Bool(_) => DataType::Boolean,
                _ => DataType::Utf8,
            };
            Field::new(name, data_type, true)
        })
        .collect();
    Schema::new(fields)
}
fn convert_to_record_batch(
    schema: &Schema,
    batch: &[HashMap<String, Value>],
) -> Result<RecordBatch, HlxError> {
    let arrays: Result<Vec<ArrayRef>, HlxError> = schema
        .fields()
        .iter()
        .map(|field| {
            let column_data: Vec<Value> = batch
                .iter()
                .map(|row| { row.get(field.name()).cloned().unwrap_or(Value::Null) })
                .collect();
            match field.data_type() {
                DataType::Utf8 => {
                    let string_data: Vec<Option<String>> = column_data
                        .into_iter()
                        .map(|v| {
                            match v {
                                Value::String(s) => Some(s),
                                _ => Some(v.to_string()),
                            }
                        })
                        .collect();
                    Ok(Arc::new(StringArray::from(string_data)) as ArrayRef)
                }
                DataType::Float64 => {
                    let float_data: Vec<Option<f64>> = column_data
                        .into_iter()
                        .map(|v| {
                            match v {
                                Value::Number(n) => Some(n),
                                Value::String(s) => s.parse().ok(),
                                _ => None,
                            }
                        })
                        .collect();
                    Ok(Arc::new(Float64Array::from(float_data)) as ArrayRef)
                }
                DataType::Int64 => {
                    let int_data: Vec<Option<i64>> = column_data
                        .into_iter()
                        .map(|v| {
                            match v {
                                Value::Number(n) => Some(n as i64),
                                Value::String(s) => s.parse().ok(),
                                _ => None,
                            }
                        })
                        .collect();
                    Ok(Arc::new(Int64Array::from(int_data)) as ArrayRef)
                }
                DataType::Boolean => {
                    let bool_data: Vec<Option<bool>> = column_data
                        .into_iter()
                        .map(|v| {
                            match v {
                                Value::Bool(b) => Some(b),
                                Value::String(s) => {
                                    match s.to_lowercase().as_str() {
                                        "true" | "1" | "yes" => Some(true),
                                        "false" | "0" | "no" => Some(false),
                                        _ => None,
                                    }
                                }
                                _ => None,
                            }
                        })
                        .collect();
                    Ok(Arc::new(arrow::array::BooleanArray::from(bool_data)) as ArrayRef)
                }
                _ => {
                    let string_data: Vec<Option<String>> = column_data
                        .into_iter()
                        .map(|v| { Some(v.to_string()) })
                        .collect();
                    Ok(Arc::new(StringArray::from(string_data)) as ArrayRef)
                }
            }
        })
        .collect();
    let arrays = arrays?;
    RecordBatch::try_new(Arc::new(schema.clone()), arrays)
        .map_err(|e| HlxError::validation_error(
            format!("Failed to create record batch: {}", e),
            "",
        ))
}
fn convert_batch_to_hashmap(batch: &RecordBatch) -> HashMap<String, Value> {
    let mut result = HashMap::new();
    for (field_idx, field) in batch.schema().fields().iter().enumerate() {
        if let Some(array) = batch
            .column(field_idx)
            .as_any()
            .downcast_ref::<StringArray>()
        {
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
        } else if let Some(array) = batch
            .column(field_idx)
            .as_any()
            .downcast_ref::<Float64Array>()
        {
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
        } else if let Some(array) = batch
            .column(field_idx)
            .as_any()
            .downcast_ref::<Int64Array>()
        {
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
        assert_eq!(schema.field(0).data_type(), & DataType::Utf8);
        assert_eq!(schema.field(1).name(), "age");
        assert_eq!(schema.field(1).data_type(), & DataType::Float64);
    }
    #[test]
    fn test_output_format_from_str() {
        assert_eq!(
            OutputFormat::from("helix").expect("Failed to parse 'helix'"),
            OutputFormat::Helix
        );
        assert_eq!(
            OutputFormat::from("hlxc").expect("Failed to parse 'hlxc'"),
            OutputFormat::Hlxc
        );
        assert_eq!(
            OutputFormat::from("compressed").expect("Failed to parse 'compressed'"),
            OutputFormat::Hlxc
        );
        assert_eq!(
            OutputFormat::from("parquet").expect("Failed to parse 'parquet'"),
            OutputFormat::Parquet
        );
        assert_eq!(
            OutputFormat::from("msgpack").expect("Failed to parse 'msgpack'"),
            OutputFormat::MsgPack
        );
        assert_eq!(
            OutputFormat::from("jsonl").expect("Failed to parse 'jsonl'"),
            OutputFormat::Jsonl
        );
        assert_eq!(
            OutputFormat::from("csv").expect("Failed to parse 'csv'"), OutputFormat::Csv
        );
        assert!(OutputFormat::from("invalid").is_err());
    }
}