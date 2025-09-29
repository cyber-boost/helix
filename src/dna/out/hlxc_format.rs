use arrow::array::*;
use arrow::datatypes::*;
use arrow::record_batch::RecordBatch;
use arrow::ipc::writer::StreamWriter;
use arrow::ipc::reader::StreamReader;
use arrow::ipc::writer::IpcWriteOptions;
use std::io::{Write, Seek, SeekFrom};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
pub use crate::dna::hel::error::HlxError;
pub use crate::dna::atp::output::OutputConfig;
pub use crate::dna::atp::output::DataWriter;

/// HLXC Format Magic Header - exactly as specified
pub const HLXC_MAGIC: &[u8; 4] = b"HLXC";
pub const HLXC_FOOTER_MAGIC: &[u8; 4] = b"\xFF\xFF\xFF\xFF";

/// HLXC Format Version
pub const HLXC_VERSION: u8 = 1;

/// HLXC File Header Structure - simplified JSON schema only
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlxcHeader {
    /// Schema fields as JSON array
    pub fields: Vec<HlxcField>,
    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlxcField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl HlxcHeader {
    pub fn new(schema: &Schema) -> Self {
        let fields: Vec<HlxcField> = schema.fields().iter().map(|field| {
            HlxcField {
                name: field.name().clone(),
                field_type: format!("{:?}", field.data_type()).to_lowercase(),
                description: None,
            }
        }).collect();

        Self {
            fields,
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = Some(metadata);
        self
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

/// HLXC Format Writer - implements the exact specification
pub struct HlxcWriter<W: Write + Seek> {
    writer: W,
    schema: Option<Schema>,
    batches: Vec<RecordBatch>,
    include_preview: bool,
    preview_rows: usize,
}

impl<W: Write + Seek> HlxcWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            schema: None,
            batches: Vec::new(),
            include_preview: true,
            preview_rows: 10,
        }
    }

    pub fn with_preview(mut self, include: bool, rows: usize) -> Self {
        self.include_preview = include;
        self.preview_rows = rows;
        self
    }

    /// Set the schema for the output
    pub fn with_schema(mut self, schema: Schema) -> Self {
        self.schema = Some(schema);
        self
    }

    /// Add a record batch to the output
    pub fn add_batch(&mut self, batch: RecordBatch) -> Result<(), HlxError> {
        // Update schema if not set
        if self.schema.is_none() {
            self.schema = Some(batch.schema().as_ref().clone());
        } else if self.schema.as_ref().unwrap() != batch.schema().as_ref() {
            return Err(HlxError::validation_error(
                "Schema mismatch between batches",
                "All batches must have the same schema"
            ));
        }

        self.batches.push(batch);
        Ok(())
    }

    /// Finalize and write the HLXC file with exact specification format
    pub fn finalize(&mut self) -> Result<(), HlxError> {
        if self.batches.is_empty() {
            return Err(HlxError::validation_error(
                "No data to write",
                "At least one record batch is required"
            ));
        }

        let schema = self.schema.as_ref().unwrap().clone();

        // Create header with schema
        let mut metadata = HashMap::new();
        metadata.insert("format_version".to_string(),
                       serde_json::Value::String("1.0".to_string()));
        metadata.insert("created_at".to_string(),
                       serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

        let header = HlxcHeader::new(&schema).with_metadata(metadata);

        // Write magic number "HLX"
        self.writer.write_all(HLXC_MAGIC)?;

        // Write version byte (0x01)
        self.writer.write_all(&[HLXC_VERSION])?;

        // Write flags byte (currently unused, set to 0)
        let flags: u8 = 0x00;
        self.writer.write_all(&[flags])?;

        // Write header JSON
        let header_json = header.to_json_bytes()?;
        let header_len = header_json.len() as u32;
        self.writer.write_all(&header_len.to_le_bytes())?;
        self.writer.write_all(&header_json)?;

        // Collect all preview rows if needed
        let preview_jsonl = if self.include_preview {
            self.extract_preview_jsonl()
        } else {
            None
        };

        // Write data as Arrow IPC stream
        let options = IpcWriteOptions::default();

        let mut stream_writer = StreamWriter::try_new_with_options(&mut self.writer, &schema, options)
            .map_err(|e| HlxError::io_error(format!("Failed to create Arrow IPC stream writer: {}", e), "Check Arrow IPC format support"))?;

        for batch in &self.batches {
            stream_writer.write(batch)
                .map_err(|e| HlxError::io_error(format!("Failed to write Arrow record batch: {}", e), "Check Arrow record batch format"))?;
        }

        stream_writer.finish()
            .map_err(|e| HlxError::io_error(format!("Failed to finalize Arrow IPC stream: {}", e), "Check stream finalization"))?;

        // Write preview footer if we have preview rows
        if let Some(jsonl) = preview_jsonl {
            eprintln!("DEBUG: Preview JSONL length: {}", jsonl.len());
            if !jsonl.is_empty() {
                eprintln!("DEBUG: Writing footer with {} bytes at position before write", self.writer.stream_position().unwrap_or(0));
                // Write footer in reverse order so magic is at the end: content + length + magic

                // Write JSONL content first
                self.writer.write_all(jsonl.as_bytes())?;
                eprintln!("DEBUG: Wrote footer content, now at position {}", self.writer.stream_position().unwrap_or(0));

                // Write footer length
                let footer_len = jsonl.len() as u32;
                self.writer.write_all(&footer_len.to_le_bytes())?;
                eprintln!("DEBUG: Wrote footer length: {}, now at position {}", footer_len, self.writer.stream_position().unwrap_or(0));

                // Write footer magic last (so it's at the very end)
                self.writer.write_all(HLXC_FOOTER_MAGIC)?;
                eprintln!("DEBUG: Wrote footer magic, final position {}", self.writer.stream_position().unwrap_or(0));
            } else {
                eprintln!("DEBUG: Preview JSONL is empty, not writing footer");
            }
        } else {
            eprintln!("DEBUG: No preview JSONL generated");
        }

        Ok(())
    }

    /// Extract preview rows as JSONL string
    fn extract_preview_jsonl(&self) -> Option<String> {
        if self.batches.is_empty() {
            return None;
        }

        let mut preview_lines = Vec::new();
        let mut rows_collected = 0;

        for batch in &self.batches {
            if rows_collected >= self.preview_rows {
                break;
            }

            let rows_in_batch = std::cmp::min(self.preview_rows - rows_collected, batch.num_rows());

            for row_idx in 0..rows_in_batch {
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
                    } else if let Some(array) = batch.column(field_idx).as_any().downcast_ref::<BooleanArray>() {
                        if array.is_valid(row_idx) {
                            let value = array.value(row_idx);
                            row_json.insert(field.name().clone(),
                                           serde_json::Value::Bool(value));
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

                let row_json_value = serde_json::Value::Object(row_json);
                if let Ok(json_str) = serde_json::to_string(&row_json_value) {
                    preview_lines.push(json_str);
                }

                rows_collected += 1;
                if rows_collected >= self.preview_rows {
                    break;
                }
            }
        }

        if preview_lines.is_empty() {
            None
        } else {
            Some(preview_lines.join("\n"))
        }
    }

}

/// HLXC Format Reader for preview functionality
pub struct HlxcReader<R: std::io::Read + Seek> {
    reader: R,
}

impl<R: std::io::Read + Seek> HlxcReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    /// Read and validate the header
    pub fn read_header(&mut self) -> Result<HlxcHeader, HlxError> {
        // Read magic number
        let mut magic = [0u8; 4];
        self.reader.read_exact(&mut magic)?;
        if magic != *HLXC_MAGIC {
            return Err(HlxError::validation_error(
                "Invalid HLXC magic number",
                "File does not appear to be a valid HLXC file"
            ));
        }

        // Read version
        let mut version = [0u8; 1];
        self.reader.read_exact(&mut version)?;
        if version[0] != HLXC_VERSION {
            return Err(HlxError::validation_error(
                format!("Unsupported HLXC version: {}", version[0]),
                "Only version 1 is supported"
            ));
        }

        // Read flags
        let mut flags = [0u8; 1];
        self.reader.read_exact(&mut flags)?;

        // Read header length
        let mut header_len_bytes = [0u8; 4];
        self.reader.read_exact(&mut header_len_bytes)?;
        let header_len = u32::from_le_bytes(header_len_bytes) as usize;

        // Read header JSON
        let mut header_bytes = vec![0u8; header_len];
        self.reader.read_exact(&mut header_bytes)?;
        let header: HlxcHeader = HlxcHeader::from_json_bytes(&header_bytes)?;

        Ok(header)
    }

    /// Get preview rows if available
    pub fn get_preview(&mut self) -> Result<Option<Vec<serde_json::Value>>, HlxError> {
        // Try to read from footer
        self.read_footer()
    }

    /// Read footer preview if available
    fn read_footer(&mut self) -> Result<Option<Vec<serde_json::Value>>, HlxError> {
        // Seek to end and look for footer magic
        let current_pos = self.reader.stream_position()?;
        eprintln!("DEBUG Reader: Current position before seeking: {}", current_pos);
        let file_size = self.reader.seek(SeekFrom::End(0))?;
        eprintln!("DEBUG Reader: File size: {}", file_size);
        if file_size < 8 {
            return Ok(None); // File too small for footer
        }

        // The footer format is: content + length (4 bytes) + magic (4 bytes)
        // So the last 8 bytes are: length (4) + magic (4)

        // Read the last 8 bytes - this should be length + magic
        self.reader.seek(SeekFrom::End(-8))?;
        let pos_after_seek = self.reader.stream_position()?;
        eprintln!("DEBUG Reader: Position after seeking to -8: {}", pos_after_seek);
        let mut footer_header = [0u8; 8];
        self.reader.read_exact(&mut footer_header)?;

        let magic = &footer_header[4..8];
        eprintln!("DEBUG Reader: Footer magic: {:?}", magic);
        eprintln!("DEBUG Reader: Expected magic: {:?}", *HLXC_FOOTER_MAGIC);
        if magic != *HLXC_FOOTER_MAGIC {
            eprintln!("DEBUG Reader: Magic mismatch, no footer found");
            return Ok(None); // No footer
        }

        let footer_len = u32::from_le_bytes(footer_header[0..4].try_into().unwrap()) as usize;
        eprintln!("DEBUG Reader: Footer length: {}", footer_len);

        // Now seek back to read the footer content
        // Footer content starts at (end - 8 - footer_len)
        let content_start = file_size - 8 - footer_len as u64;
        eprintln!("DEBUG Reader: Content should start at: {}", content_start);
        self.reader.seek(SeekFrom::Start(content_start))?;
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

    /// Check if file is compressed
    pub fn is_compressed(&mut self) -> Result<bool, HlxError> {
        // Currently no compression support, always return false
        Ok(false)
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

#[cfg(test)]
mod tests {

    use std::io::Cursor;
    use arrow::array::StringArray;

    #[test]
    fn test_hlxc_header_serialization() {
        let schema = create_arrow_schema(vec![
            ("name", DataType::Utf8),
            ("age", DataType::Int64),
        ]);

        let header = HlxcHeader::new(&schema);

        let json_bytes = header.to_json_bytes().unwrap();
        let deserialized: HlxcHeader = HlxcHeader::from_json_bytes(&json_bytes).unwrap();

        assert_eq!(header.fields.len(), deserialized.fields.len());
        assert_eq!(header.fields[0].name, "name");
        assert_eq!(header.fields[0].field_type, "utf8");
    }

    #[test]
    fn test_hlxc_writer_basic() {
        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let writer = HlxcWriter::new(cursor);

        // Just test that the writer can be created
        assert!(writer.schema.is_none());
    }

    #[test]
    fn test_hlxc_reader_magic() {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(HLXC_MAGIC);
        buffer.push(HLXC_VERSION);
        buffer.push(0x00); // flags

        let header = HlxcHeader::new(&create_arrow_schema(vec![("test", DataType::Utf8)]));
        let header_json = header.to_json_bytes().unwrap();
        let header_len = header_json.len() as u32;
        buffer.extend_from_slice(&header_len.to_le_bytes());
        buffer.extend_from_slice(&header_json);

        let cursor = Cursor::new(buffer);
        let mut reader = HlxcReader::new(cursor);
        let read_header = reader.read_header().unwrap();

        assert_eq!(read_header.fields.len(), 1);
        assert_eq!(read_header.fields[0].name, "test");
    }
}

/// Create Arrow schema from field definitions
pub fn create_arrow_schema(fields: Vec<(&str, DataType)>) -> Schema {
    let arrow_fields: Vec<Field> = fields.into_iter()
        .map(|(name, data_type)| Field::new(name, data_type, true))
        .collect();

    Schema::new(arrow_fields)
}

/// HLXC Data Writer implementation for the OutputManager
pub struct HlxcDataWriter {
    writer: Option<HlxcWriter<std::fs::File>>,
    config: OutputConfig,
    batch_count: usize,
}

impl HlxcDataWriter {
    pub fn new(config: OutputConfig) -> Self {
        Self {
            writer: None,
            config,
            batch_count: 0,
        }
    }

    fn ensure_writer(&mut self) -> Result<(), HlxError> {
        if self.writer.is_none() {
            let filename = format!("output_{:04}.hlxc", self.batch_count);
            let filepath = self.config.output_dir.join(filename);
            std::fs::create_dir_all(&self.config.output_dir)?;

            let file = std::fs::File::create(filepath)?;
            let writer = HlxcWriter::new(file)
                .with_preview(self.config.include_preview, self.config.preview_rows);

            self.writer = Some(writer);
        }
        Ok(())
    }
}

impl DataWriter for HlxcDataWriter {
    fn write_batch(&mut self, batch: RecordBatch) -> Result<(), HlxError> {
        self.ensure_writer()?;

        if let Some(writer) = &mut self.writer {
            writer.add_batch(batch)?;
        }

        Ok(())
    }

    fn finalize(&mut self) -> Result<(), HlxError> {
        if let Some(mut writer) = self.writer.take() {
            writer.finalize()?;
            self.batch_count += 1;
        }
        Ok(())
    }
}
