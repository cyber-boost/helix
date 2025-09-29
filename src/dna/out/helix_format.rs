use arrow::array::*;
use arrow::datatypes::*;
use arrow::record_batch::RecordBatch;
use arrow::ipc::writer::*;
use arrow::ipc::reader::*;
use std::io::{Write, Seek, SeekFrom};
use std::collections::HashMap;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
pub use crate::dna::hel::error::HlxError;

/// Helix Data Format (.helix files) Magic Header
pub const HELIX_DATA_MAGIC: &[u8; 4] = b"HLX\x01";
pub const HELIX_DATA_FOOTER_MAGIC: &[u8; 4] = b"\xFF\xFF\xFF\xFF";

/// Helix Data Format Version
pub const HELIX_DATA_VERSION: u8 = 1;

/// Helix Data File (.helix) Header Structure
/// This is for binary DATA output files, not configuration files.
/// For configuration files, see hlx_config_format.rs (.hlx) and hlxb_config_format.rs (.hlxb)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlxHeader {
    /// Schema fields (simplified representation)
    pub fields: Vec<HlxField>,
    /// Optional metadata about the run
    pub metadata: HashMap<String, serde_json::Value>,
    /// Compression flags
    pub flags: u8,
    /// Number of rows in this file
    pub row_count: u64,
    /// Optional preview rows (first N rows as JSONL)
    pub preview_rows: Option<Vec<serde_json::Value>>,
}

/// Simplified field representation for HLX header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlxField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
}

impl HlxHeader {
    pub fn new(schema: &Schema, metadata: HashMap<String, serde_json::Value>) -> Self {
        let fields: Vec<HlxField> = schema.fields().iter().map(|field| {
            HlxField {
                name: field.name().clone(),
                field_type: format!("{:?}", field.data_type()).to_lowercase(),
            }
        }).collect();

        Self {
            fields,
            metadata,
            flags: 0,
            row_count: 0,
            preview_rows: None,
        }
    }

    /// Set compression flag
    pub fn with_compression(mut self, compressed: bool) -> Self {
        if compressed {
            self.flags |= 0x01;
        } else {
            self.flags &= !0x01;
        }
        self
    }

    /// Set row count
    pub fn with_row_count(mut self, count: u64) -> Self {
        self.row_count = count;
        self
    }

    /// Set preview rows (first N rows as JSON)
    pub fn with_preview(mut self, preview: Vec<serde_json::Value>) -> Self {
        self.preview_rows = Some(preview);
        self
    }

    /// Check if compression is enabled
    pub fn is_compressed(&self) -> bool {
        (self.flags & 0x01) != 0
    }

    /// Serialize header to JSON bytes
    pub fn to_json_bytes(&self) -> Result<Vec<u8>, HlxError> {
        serde_json::to_vec(self).map_err(|e| HlxError::json_error(e.to_string(), ""))
    }

    /// Deserialize header from JSON bytes
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, HlxError> {
        serde_json::from_slice(bytes).map_err(|e| HlxError::json_error(e.to_string(), ""))
    }
}

/// HLX Format Writer
pub struct HlxWriter<W: Write + Seek> {
    writer: W,
    header: Option<HlxHeader>,
    schema: Option<Arc<Schema>>,
    batches: Vec<RecordBatch>,
    compression_enabled: bool,
}

impl<W: Write + Seek> HlxWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            header: None,
            schema: None,
            batches: Vec::new(),
            compression_enabled: false,
        }
    }

    pub fn with_compression(mut self, enabled: bool) -> Self {
        self.compression_enabled = enabled;
        self
    }

    /// Set the schema for the output
    pub fn with_schema(mut self, schema: Arc<Schema>) -> Self {
        self.schema = Some(schema);
        self
    }

    /// Add a record batch to the output
    pub fn add_batch(&mut self, batch: RecordBatch) -> Result<(), HlxError> {
        // Update schema if not set
        if self.schema.is_none() {
            self.schema = Some(batch.schema().clone());
        } else if self.schema.as_ref().unwrap().as_ref() != batch.schema().as_ref() {
            return Err(HlxError::validation_error(
                "Schema mismatch between batches",
                "All batches must have the same schema"
            ));
        }

        self.batches.push(batch);
        Ok(())
    }

    /// Finalize and write the HLX file
    pub fn finalize(&mut self) -> Result<(), HlxError> {
        if self.batches.is_empty() {
            return Err(HlxError::validation_error(
                "No data to write",
                "At least one record batch is required"
            ));
        }

        let schema = self.schema.as_ref().unwrap().clone();
        let total_rows = self.batches.iter().map(|b| b.num_rows()).sum::<usize>() as u64;

        // Create header with metadata
        let mut metadata = HashMap::new();
        metadata.insert("format_version".to_string(),
                       serde_json::Value::String("1.0".to_string()));
        metadata.insert("created_at".to_string(),
                       serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

        let mut header = HlxHeader::new(&schema, metadata)
            .with_compression(self.compression_enabled)
            .with_row_count(total_rows);

        // Add preview rows (first 10 rows)
        if let Some(first_batch) = self.batches.first() {
            let preview_count = std::cmp::min(10, first_batch.num_rows());
            let preview_rows = extract_preview_rows(first_batch, preview_count);
            header = header.with_preview(preview_rows);
        }

        // Write magic number and header
        self.writer.write_all(HELIX_DATA_MAGIC)?;
        let header_bytes = header.to_json_bytes()?;
        let header_len = header_bytes.len() as u32;
        self.writer.write_all(&header_len.to_le_bytes())?;
        self.writer.write_all(&header_bytes)?;

        // Write data as Arrow IPC stream
        let mut options = IpcWriteOptions::default();
        if self.compression_enabled {
            // Try to enable ZSTD compression - this API may vary by Arrow version
            // For now, we'll use the default options and note that compression
            // needs to be properly configured for the specific Arrow version
            eprintln!("Warning: Compression enabled but not yet implemented for this Arrow version");
        }

        let mut stream_writer = StreamWriter::try_new_with_options(&mut self.writer, &schema, options)
            .map_err(|e| HlxError::io_error(format!("Failed to create Arrow IPC stream writer: {}", e), "Check Arrow IPC format support"))?;

        for batch in &self.batches {
            stream_writer.write(batch)
                .map_err(|e| HlxError::io_error(format!("Failed to write Arrow record batch: {}", e), "Check Arrow record batch format"))?;
        }

        stream_writer.finish()
            .map_err(|e| HlxError::io_error(format!("Failed to finalize Arrow IPC stream: {}", e), "Check stream finalization"))?;

        // Write preview footer if we have preview rows
        if let Some(preview_rows) = &header.preview_rows {
            if !preview_rows.is_empty() {
                // Write footer magic
                self.writer.write_all(HELIX_DATA_FOOTER_MAGIC)?;

                // Write preview as JSONL
                let preview_jsonl = preview_rows.iter()
                    .map(|row| serde_json::to_string(row).unwrap_or_default())
                    .collect::<Vec<_>>()
                    .join("\n");

                let footer_len = preview_jsonl.len() as u32;
                self.writer.write_all(&footer_len.to_le_bytes())?;
                self.writer.write_all(preview_jsonl.as_bytes())?;
            }
        }

        Ok(())
    }
}

/// Extract preview rows from a record batch
fn extract_preview_rows(batch: &RecordBatch, count: usize) -> Vec<serde_json::Value> {
    let mut preview_rows = Vec::new();

    for row_idx in 0..count {
        let mut row_json = serde_json::Map::new();

        for (field_idx, field) in batch.schema().fields().iter().enumerate() {
            if let Some(array) = batch.column(field_idx).as_any().downcast_ref::<StringArray>() {
                if array.is_valid(row_idx) {
                    let value = array.value(row_idx);
                    row_json.insert(field.name().clone(),
                                   serde_json::Value::String(value.to_string()));
                } else {
                    row_json.insert(field.name().clone(), serde_json::Value::Null);
                }
            } else if let Some(array) = batch.column(field_idx).as_any().downcast_ref::<Float64Array>() {
                if array.is_valid(row_idx) {
                    let value = array.value(row_idx);
                    row_json.insert(field.name().clone(),
                                   serde_json::Value::Number(serde_json::Number::from_f64(value).unwrap_or(serde_json::Number::from(0))));
                } else {
                    row_json.insert(field.name().clone(), serde_json::Value::Null);
                }
            } else if let Some(array) = batch.column(field_idx).as_any().downcast_ref::<Int64Array>() {
                if array.is_valid(row_idx) {
                    let value = array.value(row_idx);
                    row_json.insert(field.name().clone(),
                                   serde_json::Value::Number(serde_json::Number::from(value)));
                } else {
                    row_json.insert(field.name().clone(), serde_json::Value::Null);
                }
            } else {
                // For other array types, check if valid and convert to string
                if batch.column(field_idx).is_valid(row_idx) {
                    let value_str = format!("{:?}", batch.column(field_idx));
                    row_json.insert(field.name().clone(),
                                   serde_json::Value::String(value_str));
                } else {
                    row_json.insert(field.name().clone(), serde_json::Value::Null);
                }
            }
        }

        preview_rows.push(serde_json::Value::Object(row_json));
    }

    preview_rows
}

/// HLX Format Reader for preview functionality
pub struct HlxReader<R: std::io::Read + Seek> {
    reader: R,
    header: Option<HlxHeader>,
}

impl<R: std::io::Read + Seek> HlxReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            header: None,
        }
    }

    /// Read and validate the header
    pub fn read_header(&mut self) -> Result<&HlxHeader, HlxError> {
        if self.header.is_some() {
            return Ok(self.header.as_ref().unwrap());
        }

        // Read magic number
        let mut magic = [0u8; 4];
        self.reader.read_exact(&mut magic)?;
        if magic != *HELIX_DATA_MAGIC {
            return Err(HlxError::validation_error(
                "Invalid Helix data magic number",
                "File does not appear to be a valid .helix data file"
            ));
        }

        // Read header length
        let mut header_len_bytes = [0u8; 4];
        self.reader.read_exact(&mut header_len_bytes)?;
        let header_len = u32::from_le_bytes(header_len_bytes) as usize;

        // Read header JSON
        let mut header_bytes = vec![0u8; header_len];
        self.reader.read_exact(&mut header_bytes)?;
        let header: HlxHeader = HlxHeader::from_json_bytes(&header_bytes)?;

        self.header = Some(header);
        Ok(self.header.as_ref().unwrap())
    }

    /// Get preview rows if available
    pub fn get_preview(&mut self) -> Result<Option<Vec<serde_json::Value>>, HlxError> {
        let header = self.read_header()?;

        if let Some(preview_rows) = &header.preview_rows {
            return Ok(Some(preview_rows.clone()));
        }

        // Try to read from footer
        if let Some(footer_rows) = self.read_footer()? {
            return Ok(Some(footer_rows));
        }

        Ok(None)
    }

    /// Read footer preview if available
    fn read_footer(&mut self) -> Result<Option<Vec<serde_json::Value>>, HlxError> {
        // Seek to end and look for footer magic
        let file_size = self.reader.seek(SeekFrom::End(0))?;
        if file_size < 8 {
            return Ok(None); // File too small for footer
        }

        // Read last 8 bytes (footer magic + length)
        self.reader.seek(SeekFrom::End(-8))?;
        let mut footer_header = [0u8; 8];
        self.reader.read_exact(&mut footer_header)?;

        let magic = &footer_header[0..4];
        if magic != *HELIX_DATA_FOOTER_MAGIC {
            return Ok(None); // No footer
        }

        let footer_len = u32::from_le_bytes(footer_header[4..8].try_into().unwrap()) as usize;

        // Read footer content
        self.reader.seek(SeekFrom::End(-8 - footer_len as i64))?;
        let mut footer_bytes = vec![0u8; footer_len];
        self.reader.read_exact(&mut footer_bytes)?;

        let footer_jsonl = String::from_utf8(footer_bytes)
            .map_err(|_| HlxError::validation_error("Invalid UTF-8 in footer", ""))?;

        let rows: Vec<serde_json::Value> = footer_jsonl
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).unwrap_or(serde_json::Value::Null))
            .collect();

        Ok(Some(rows))
    }

    /// Get schema information
    pub fn get_schema(&mut self) -> Result<&HlxHeader, HlxError> {
        self.read_header()
    }

    /// Read Arrow IPC data as record batches
    pub fn read_batches(&mut self) -> Result<Vec<RecordBatch>, HlxError> {
        // Skip header first
        self.read_header()?;

        // Create a reader from the current position (after header)
        let reader = StreamReader::try_new(&mut self.reader, Default::default())
            .map_err(|e| HlxError::io_error(format!("Failed to create Arrow IPC stream reader: {}", e), "Check Arrow IPC stream format"))?;

        // Collect all batches
        let mut batches = Vec::new();
        for batch_result in reader {
            let batch = batch_result
                .map_err(|e| HlxError::io_error(format!("Failed to read Arrow record batch: {}", e), "Check Arrow IPC data integrity"))?;
            batches.push(batch);
        }

        Ok(batches)
    }
}

/// Convert Helix Value to Arrow Array
pub fn value_to_arrow_array(field: &Field, values: Vec<crate::dna::atp::value::Value>) -> Result<ArrayRef, HlxError> {
    match field.data_type() {
        DataType::Utf8 => {
            let string_values: Vec<Option<String>> = values.into_iter()
                .map(|v| match v {
                    crate::dna::atp::value::Value::String(s) => Some(s),
                    _ => Some(v.to_string()),
                })
                .collect();

            let array = StringArray::from(string_values);
            Ok(Arc::new(array))
        }
        DataType::Float64 => {
            let float_values: Vec<Option<f64>> = values.into_iter()
                .map(|v| match v {
                    crate::dna::atp::value::Value::Number(n) => Some(n),
                    _ => v.to_string().parse().ok(),
                })
                .collect();

            let array = Float64Array::from(float_values);
            Ok(Arc::new(array))
        }
        DataType::Int64 => {
            let int_values: Vec<Option<i64>> = values.into_iter()
                .map(|v| match v {
                    crate::dna::atp::value::Value::Number(n) => Some(n as i64),
                    _ => v.to_string().parse().ok(),
                })
                .collect();

            let array = Int64Array::from(int_values);
            Ok(Arc::new(array))
        }
        _ => {
            // Default to string representation
            let string_values: Vec<Option<String>> = values.into_iter()
                .map(|v| Some(v.to_string()))
                .collect();

            let array = StringArray::from(string_values);
            Ok(Arc::new(array))
        }
    }
}

/// Create Arrow schema from field definitions
pub fn create_arrow_schema(fields: Vec<(&str, DataType)>) -> Schema {
    let arrow_fields: Vec<Field> = fields.into_iter()
        .map(|(name, data_type)| Field::new(name, data_type, true))
        .collect();

    Schema::new(arrow_fields)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_hlx_header_serialization() {
        let mut metadata = HashMap::new();
        metadata.insert("test".to_string(), serde_json::Value::String("value".to_string()));

        let schema = create_arrow_schema(vec![
            ("name", DataType::Utf8),
            ("age", DataType::Int64),
        ]);

        let header = HlxHeader::new(&schema, metadata)
            .with_compression(true)
            .with_row_count(100);

        let json_bytes = header.to_json_bytes().unwrap();
        let deserialized: HlxHeader = HlxHeader::from_json_bytes(&json_bytes).unwrap();

        assert_eq!(header.row_count, deserialized.row_count);
        assert!(header.is_compressed());
    }

    #[test]
    fn test_hlx_writer_basic() {
        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let mut writer = HlxWriter::new(cursor);

        // This test would need actual data to be meaningful
        // For now, just test that the writer can be created
        assert!(writer.header.is_none());
    }
}
