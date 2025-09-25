// HLXC format comprehensive tests
use helix::output::*;
use helix::output::hlxc_format::{HlxcWriter, HlxcReader, HLXC_MAGIC, HLXC_VERSION};
use helix::value::Value;
use arrow::datatypes::*;
use arrow::record_batch::RecordBatch;
use arrow::array::*;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use tempfile::TempDir;

/// Test basic HLXC writer functionality
#[test]
fn test_basic_hlxc_writer() {
let mut buffer = Vec::new();
let cursor = Cursor::new(&mut buffer);
let mut writer = HlxcWriter::new(cursor);

// Create a simple schema
let schema = create_arrow_schema(vec![
("name", DataType::Utf8),
("age", DataType::Int64),
("active", DataType::Boolean),
]);

// Create test data
let names = StringArray::from(vec!["Alice", "Bob", "Charlie"]);
let ages = Int64Array::from(vec![25, 30, 35]);
let active = BooleanArray::from(vec![true, false, true]);

let batch = RecordBatch::try_new(
Arc::new(schema),
vec![Arc::new(names), Arc::new(ages), Arc::new(active)],
).expect("Failed to create record batch");

writer.add_batch(batch).expect("Failed to add batch");
let result = writer.finalize();
if let Err(e) = &result {
    panic!("Finalize failed: {:?}", e);
}
result.expect("Failed to finalize");

// Debug: print buffer length and first few bytes
println!("Buffer length: {}", buffer.len());
if buffer.len() >= 10 {
    println!("First 10 bytes: {:?}", &buffer[0..10]);
}

// Verify the buffer contains HLXC data
assert!(!buffer.is_empty(), "Buffer should not be empty");
assert!(buffer.len() >= 6, "Buffer should have at least 6 bytes");
assert_eq!(&buffer[0..4], HLXC_MAGIC, "Magic number mismatch");
assert_eq!(buffer[4], HLXC_VERSION, "Version byte mismatch"); // version byte at index 4
}

/// Test HLXC reader functionality
#[test]
fn test_hlxc_reader() {
    let mut buffer = Vec::new();
    let cursor = Cursor::new(&mut buffer);
    let mut writer = HlxcWriter::new(cursor).with_preview(false, 0); // Disable preview for now

    // Create test data
    let schema = create_arrow_schema(vec![
    ("id", DataType::Int64),
    ("value", DataType::Utf8),
    ]);

    let ids = Int64Array::from(vec![1, 2, 3]);
    let values = StringArray::from(vec!["test1", "test2", "test3"]);

    let batch = RecordBatch::try_new(
    Arc::new(schema.clone()),
    vec![Arc::new(ids), Arc::new(values)],
    ).expect("Failed to create record batch");

    writer.add_batch(batch).expect("Failed to add batch");
    writer.finalize().expect("Failed to finalize");

    // Now read it back
    let read_cursor = Cursor::new(&buffer);
    let mut reader = HlxcReader::new(read_cursor);

    // Test that reader can read header
    let header = reader.read_header().expect("Failed to read header");
    assert_eq!(header.fields.len(), 2);
    assert_eq!(header.fields[0].name, "id");
    assert_eq!(header.fields[1].name, "value");

    // Preview should be None since we disabled it
    let preview = reader.get_preview().expect("Failed to get preview");
    assert!(preview.is_none(), "Preview should be None when disabled");
}

/// Test HLXC compression functionality
#[test]
fn test_hlxc_compression() {
let mut buffer_uncompressed = Vec::new();
let mut buffer_compressed = Vec::new();

// Create uncompressed writer
let cursor_uncompressed = Cursor::new(&mut buffer_uncompressed);
let mut writer_uncompressed = HlxcWriter::new(cursor_uncompressed).with_compression(false);

// Create compressed writer
let cursor_compressed = Cursor::new(&mut buffer_compressed);
let mut writer_compressed = HlxcWriter::new(cursor_compressed).with_compression(true);

// Create large test data to see compression effect
let schema = create_arrow_schema(vec![
("data", DataType::Utf8),
]);

let large_data: Vec<String> = (0..1000)
.map(|i| format!("This is test data number {} with some repetitive content that should compress well", i))
.collect();
let data_array = StringArray::from(large_data);

let batch = RecordBatch::try_new(
Arc::new(schema),
vec![Arc::new(data_array)],
).expect("Failed to create record batch");

writer_uncompressed.add_batch(batch.clone()).expect("Failed to add batch to uncompressed");
writer_compressed.add_batch(batch).expect("Failed to add batch to compressed");

writer_uncompressed.finalize().expect("Failed to finalize uncompressed");
writer_compressed.finalize().expect("Failed to finalize compressed");

// Check compression flags
let mut compressed_reader = HlxcReader::new(Cursor::new(&buffer_compressed));
let is_compressed = compressed_reader.is_compressed().expect("Failed to check compression");
assert!(is_compressed);

let mut uncompressed_reader = HlxcReader::new(Cursor::new(&buffer_uncompressed));
let is_not_compressed = uncompressed_reader.is_compressed().expect("Failed to check compression");
assert!(!is_not_compressed);
}

/// Test HLXC preview functionality
#[test]
fn test_hlxc_preview() {
let mut buffer = Vec::new();
let cursor = Cursor::new(&mut buffer);
let mut writer = HlxcWriter::new(cursor)
.with_preview(true, 5); // Include preview with 5 rows

let schema = create_arrow_schema(vec![
("name", DataType::Utf8),
("score", DataType::Float64),
]);

let names = StringArray::from(vec!["Alice", "Bob", "Charlie", "David", "Eve"]);
let scores = Float64Array::from(vec![95.5, 87.2, 91.8, 88.9, 93.3]);

let batch = RecordBatch::try_new(
Arc::new(schema),
vec![Arc::new(names), Arc::new(scores)],
).expect("Failed to create record batch");

writer.add_batch(batch).expect("Failed to add batch");
writer.finalize().expect("Failed to finalize");

// Read back and check preview
let read_cursor = Cursor::new(&buffer);
let mut reader = HlxcReader::new(read_cursor);

// Must read header first to position cursor correctly
let _header = reader.read_header().expect("Failed to read header");

let preview = reader.get_preview().expect("Failed to get preview");
assert!(preview.is_some());

let preview_data = preview.unwrap();
assert_eq!(preview_data.len(), 5); // Should have 5 preview rows

// Check that preview contains expected data
let first_row = &preview_data[0];
if let serde_json::Value::Object(obj) = first_row {
assert!(obj.contains_key("name"));
assert!(obj.contains_key("score"));
} else {
panic!("Preview row should be an object");
}
}

/// Test HLXC with OutputManager integration
#[test]
fn test_hlxc_output_manager_integration() {
let temp_dir = TempDir::new().expect("Failed to create temp dir");

let mut config = OutputConfig::default();
config.output_dir = temp_dir.path().to_path_buf();
config.formats = vec![OutputFormat::Hlxc];
config.compression.enabled = true;

let mut manager = OutputManager::new(config);

// Add some test data
let mut row1 = HashMap::new();
row1.insert("name".to_string(), Value::String("Alice".to_string()));
row1.insert("age".to_string(), Value::Number(30.0));
row1.insert("active".to_string(), Value::Bool(true));

let mut row2 = HashMap::new();
row2.insert("name".to_string(), Value::String("Bob".to_string()));
row2.insert("age".to_string(), Value::Number(25.0));
row2.insert("active".to_string(), Value::Bool(false));

manager.add_row(row1).expect("Failed to add row 1");
manager.add_row(row2).expect("Failed to add row 2");
manager.finalize_all().expect("Failed to finalize");

// Check that file was created
let output_files = manager.get_output_files();
assert_eq!(output_files.len(), 1);

let hlxc_file = &output_files[0];
assert!(hlxc_file.exists());
assert!(hlxc_file.extension().unwrap() == "hlxc");

// Verify we can read it back
let mut reader = HlxcReader::new(std::fs::File::open(hlxc_file).expect("Failed to open file"));
let header = reader.read_header().expect("Failed to read header");
assert_eq!(header.fields.len(), 3);

let preview = reader.get_preview().expect("Failed to get preview");
assert!(preview.is_some());
}

/// Test HLXC with different data types
#[test]
fn test_hlxc_data_types() {
let mut buffer = Vec::new();
let cursor = Cursor::new(&mut buffer);
let mut writer = HlxcWriter::new(cursor);

let schema = create_arrow_schema(vec![
("string_col", DataType::Utf8),
("int_col", DataType::Int64),
("float_col", DataType::Float64),
("bool_col", DataType::Boolean),
]);

let strings = StringArray::from(vec!["hello", "world", "test"]);
let ints = Int64Array::from(vec![42, 1337, -1]);
let floats = Float64Array::from(vec![3.14, 2.71, 1.41]);
let bools = BooleanArray::from(vec![true, false, true]);

let batch = RecordBatch::try_new(
Arc::new(schema),
vec![Arc::new(strings), Arc::new(ints), Arc::new(floats), Arc::new(bools)],
).expect("Failed to create record batch");

writer.add_batch(batch).expect("Failed to add batch");
writer.finalize().expect("Failed to finalize");

// Read back and verify
let read_cursor = Cursor::new(&buffer);
let mut reader = HlxcReader::new(read_cursor);

let header = reader.read_header().expect("Failed to read header");
assert_eq!(header.fields.len(), 4);

let preview = reader.get_preview().expect("Failed to get preview");
assert!(preview.is_some());
}

/// Test HLXC error handling
#[test]
fn test_hlxc_error_handling() {
// Test empty writer
let mut buffer = Vec::new();
let cursor = Cursor::new(&mut buffer);
let mut writer = HlxcWriter::new(cursor);

let result = writer.finalize();
assert!(result.is_err()); // Should fail with no data

// Test invalid magic number
let invalid_data = b"INVALID";
let mut reader = HlxcReader::new(Cursor::new(invalid_data));
let result = reader.read_header();
assert!(result.is_err()); // Should fail with invalid magic

// Test schema mismatch
let mut buffer = Vec::new();
let cursor = Cursor::new(&mut buffer);
let mut writer = HlxcWriter::new(cursor);

let schema1 = create_arrow_schema(vec![("col1", DataType::Utf8)]);
let schema2 = create_arrow_schema(vec![("col2", DataType::Int64)]); // Different schema

let batch1 = RecordBatch::try_new(
Arc::new(schema1),
vec![Arc::new(StringArray::from(vec!["test"]))],
).expect("Failed to create batch 1");

let batch2 = RecordBatch::try_new(
Arc::new(schema2),
vec![Arc::new(Int64Array::from(vec![123]))],
).expect("Failed to create batch 2");

writer.add_batch(batch1).expect("Failed to add first batch");
let result = writer.add_batch(batch2);
assert!(result.is_err()); // Should fail with schema mismatch
}

/// Test HLXC round-trip consistency
#[test]
fn test_hlxc_round_trip() {
let temp_dir = TempDir::new().expect("Failed to create temp dir");
let file_path = temp_dir.path().join("test.hlxc");

// Original data
let original_data = vec![
("Alice", 25, true, 95.5),
("Bob", 30, false, 87.2),
("Charlie", 35, true, 91.8),
];

// Write data
{
let file = std::fs::File::create(&file_path).expect("Failed to create file");
let mut writer = HlxcWriter::new(file).with_compression(true);

let schema = create_arrow_schema(vec![
("name", DataType::Utf8),
("age", DataType::Int64),
("active", DataType::Boolean),
("score", DataType::Float64),
]);

let names: Vec<&str> = original_data.iter().map(|(n, _, _, _)| *n).collect();
let ages: Vec<i64> = original_data.iter().map(|(_, a, _, _)| *a).collect();
let actives: Vec<bool> = original_data.iter().map(|(_, _, a, _)| *a).collect();
let scores: Vec<f64> = original_data.iter().map(|(_, _, _, s)| *s).collect();

let batch = RecordBatch::try_new(
Arc::new(schema),
vec![
    Arc::new(StringArray::from(names)),
    Arc::new(Int64Array::from(ages)),
    Arc::new(BooleanArray::from(actives)),
    Arc::new(Float64Array::from(scores)),
],
).expect("Failed to create record batch");

writer.add_batch(batch).expect("Failed to add batch");
writer.finalize().expect("Failed to finalize");
}

// Read data back
{
let file = std::fs::File::open(&file_path).expect("Failed to open file");
let mut reader = HlxcReader::new(file);

let header = reader.read_header().expect("Failed to read header");
assert_eq!(header.fields.len(), 4);

let preview = reader.get_preview().expect("Failed to get preview");
assert!(preview.is_some());

let preview_data = preview.unwrap();
assert_eq!(preview_data.len(), original_data.len());

// Verify data integrity through preview
for (i, row) in preview_data.iter().enumerate() {
if let serde_json::Value::Object(obj) = row {
    let expected = &original_data[i];
    assert_eq!(obj["name"], expected.0);
    assert_eq!(obj["age"], expected.1);
    assert_eq!(obj["active"], expected.2);
    assert_eq!(obj["score"], expected.3);
}
}
}
}

/// Test HLXC with large datasets
#[test]
fn test_hlxc_large_dataset() {
let mut buffer = Vec::new();
let cursor = Cursor::new(&mut buffer);
let mut writer = HlxcWriter::new(cursor).with_compression(true);

let schema = create_arrow_schema(vec![
("id", DataType::Int64),
("data", DataType::Utf8),
]);

// Create a larger dataset
let ids: Vec<i64> = (0..1000).collect();
let data: Vec<String> = (0..1000)
.map(|i| format!("Data entry number {} with some additional text to make it larger", i))
.collect();

let batch = RecordBatch::try_new(
Arc::new(schema),
vec![
Arc::new(Int64Array::from(ids)),
Arc::new(StringArray::from(data)),
],
).expect("Failed to create record batch");

writer.add_batch(batch).expect("Failed to add batch");
writer.finalize().expect("Failed to finalize");

// Verify we can read it back
let read_cursor = Cursor::new(&buffer);
let mut reader = HlxcReader::new(read_cursor);

let header = reader.read_header().expect("Failed to read header");
assert_eq!(header.fields.len(), 2);

let preview = reader.get_preview().expect("Failed to get preview");
assert!(preview.is_some());
assert_eq!(preview.unwrap().len(), 10); // Default preview size
}

/// Test HLXC file operations with temporary files
#[test]
fn test_hlxc_file_operations() {
let temp_dir = TempDir::new().expect("Failed to create temp dir");
let file_path = temp_dir.path().join("test.hlxc");

// Write to file
{
let file = std::fs::File::create(&file_path).expect("Failed to create file");
let mut writer = HlxcWriter::new(file);

let schema = create_arrow_schema(vec![("test", DataType::Utf8)]);
let batch = RecordBatch::try_new(
Arc::new(schema),
vec![Arc::new(StringArray::from(vec!["Hello, World!"]))],
).expect("Failed to create batch");

writer.add_batch(batch).expect("Failed to add batch");
writer.finalize().expect("Failed to finalize");
}

// Verify file exists and has content
assert!(file_path.exists());
let metadata = std::fs::metadata(&file_path).expect("Failed to get metadata");
assert!(metadata.len() > 0);

// Read from file
{
let file = std::fs::File::open(&file_path).expect("Failed to open file");
let mut reader = HlxcReader::new(file);

let header = reader.read_header().expect("Failed to read header");
assert_eq!(header.fields[0].name, "test");

let preview = reader.get_preview().expect("Failed to get preview");
assert!(preview.is_some());
}
}

/// Helper function to create Arrow schema
fn create_arrow_schema(fields: Vec<(&str, DataType)>) -> Schema {
let arrow_fields: Vec<Field> = fields
.into_iter()
.map(|(name, data_type)| Field::new(name, data_type, true))
.collect();

Schema::new(arrow_fields)
}
