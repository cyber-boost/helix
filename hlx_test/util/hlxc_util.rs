// HLXC Compressed Data File Utilities
// Demonstrates how to create, read, and work with .hlxc compressed data files

use helix::output::{
    hlxc_format::{HlxcWriter, HlxcReader, HLXC_MAGIC, HLXC_VERSION},
    OutputFormat, OutputManager, DataWriter
};
use arrow::datatypes::*;
use arrow::record_batch::RecordBatch;
use arrow::array::*;
use std::fs::File;
use std::io::Cursor;
use std::sync::Arc;

/// Create sample data for HLXC compression testing
pub fn create_sample_record_batch() -> Result<RecordBatch, Box<dyn std::error::Error>> {
    println!("🏗️  Creating sample record batch for HLXC testing");

    // Define schema
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, false),
        Field::new("value", DataType::Float64, true),
        Field::new("category", DataType::Utf8, false),
        Field::new("active", DataType::Boolean, false),
        Field::new("tags", DataType::Utf8, true), // JSON string for array
    ]));

    // Create sample data
    let ids = Int64Array::from(vec![1, 2, 3, 4, 5]);
    let names = StringArray::from(vec!["Alice", "Bob", "Charlie", "Diana", "Eve"]);
    let values = Float64Array::from(vec![Some(100.5), Some(250.0), None, Some(75.25), Some(300.0)]);
    let categories = StringArray::from(vec!["A", "B", "A", "C", "B"]);
    let active = BooleanArray::from(vec![true, true, false, true, false]);
    let tags = StringArray::from(vec![
        Some(r#"["premium", "active"]"#),
        Some(r#"["standard"]"#),
        None,
        Some(r#"["budget", "trial"]"#),
        Some(r#"["enterprise", "vip"]"#),
    ]);

    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(ids),
            Arc::new(names),
            Arc::new(values),
            Arc::new(categories),
            Arc::new(active),
            Arc::new(tags),
        ],
    )?;

    println!("✅ Created record batch with {} rows and {} columns",
            batch.num_rows(), batch.num_columns());

    Ok(batch)
}

/// Create an HLXC file from record batches
pub fn create_hlxc_file<P: AsRef<std::path::Path>>(
    path: P,
    batches: Vec<RecordBatch>,
    enable_compression: bool
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📦 Creating HLXC file: {:?}", path.as_ref());

    let file = File::create(&path)?;
    let mut writer = HlxcWriter::new(file);

    if enable_compression {
        writer = writer.with_compression(true);
    }

    // Add metadata
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("created_by".to_string(), serde_json::json!("hlx_test_util"));
    metadata.insert("compression_enabled".to_string(), serde_json::json!(enable_compression));
    metadata.insert("batch_count".to_string(), serde_json::json!(batches.len()));

    writer = writer.with_metadata(metadata);

    // Add preview (first 3 rows)
    writer = writer.with_preview(true, 3);

    // Write all batches
    for (i, batch) in batches.iter().enumerate() {
        println!("  Writing batch {} with {} rows", i + 1, batch.num_rows());
        writer.add_batch(batch.clone())?;
    }

    // Finalize the file
    writer.finalize()?;
    println!("✅ HLXC file created successfully");

    Ok(())
}

/// Read and analyze an HLXC file
pub fn analyze_hlxc_file<P: AsRef<std::path::Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Analyzing HLXC file: {:?}", path.as_ref());

    let file = File::open(&path)?;
    let mut reader = HlxcReader::new(file);

    // Read header
    let header = reader.read_header()?;
    println!("📋 Header Information:");
    println!("  - Fields: {}", header.fields.len());
    for field in &header.fields {
        println!("    * {} ({})", field.name, field.data_type);
    }

    if let Some(metadata) = header.metadata {
        println!("  - Metadata: {}", serde_json::to_string_pretty(&metadata)?);
    }

    // Check compression
    let is_compressed = reader.is_compressed()?;
    println!("  - Compression: {}", if is_compressed { "Enabled (ZSTD)" } else { "Disabled" });

    // Read preview
    if let Some(preview) = reader.get_preview()? {
        println!("  - Preview rows: {}", preview.len());
        println!("  - Sample preview data:");
        for (i, row) in preview.iter().enumerate() {
            println!("    Row {}: {}", i + 1, serde_json::to_string(row)?);
        }
    } else {
        println!("  - Preview: Not available");
    }

    // Get file size
    let file_size = std::fs::metadata(&path)?.len();
    println!("  - File size: {} bytes ({:.2} KB)", file_size, file_size as f64 / 1024.0);

    println!("✅ HLXC file analysis complete");
    Ok(())
}

/// Compare file sizes: JSON vs HLXC
pub fn compare_compression_efficiency<P: AsRef<std::path::Path>>(
    json_path: P,
    hlxc_path: P
) -> Result<(), Box<dyn std::error::Error>> {
    println!("⚖️  Comparing compression efficiency");

    let json_size = std::fs::metadata(&json_path)?.len();
    let hlxc_size = std::fs::metadata(&hlxc_path)?.len();

    let ratio = json_size as f64 / hlxc_size as f64;
    let savings = ((json_size - hlxc_size) as f64 / json_size as f64) * 100.0;

    println!("📏 File Size Comparison:");
    println!("  - JSON: {} bytes ({:.2} KB)", json_size, json_size as f64 / 1024.0);
    println!("  - HLXC: {} bytes ({:.2} KB)", hlxc_size, hlxc_size as f64 / 1024.0);
    println!("  - Compression ratio: {:.2}x", ratio);
    println!("  - Space savings: {:.1}%", savings);

    if ratio > 1.5 {
        println!("✅ Excellent compression! HLXC is {:.1}x smaller than JSON", ratio);
    } else if ratio > 1.2 {
        println!("👍 Good compression! HLXC is {:.1}x smaller than JSON", ratio);
    } else {
        println!("🤔 Modest compression. Consider different data patterns for better results.");
    }

    Ok(())
}

/// Create HLXC file using OutputManager (recommended approach)
pub fn create_hlxc_via_output_manager<P: AsRef<std::path::Path>>(
    output_path: P,
    batches: Vec<RecordBatch>
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Creating HLXC via OutputManager: {:?}", output_path.as_ref());

    // Create output manager
    let mut manager = OutputManager::new();

    // Initialize HLXC writer
    manager.initialize_writer(OutputFormat::Hlxc, output_path.as_ref())?;

    // Write batches
    for (i, batch) in batches.iter().enumerate() {
        println!("  Writing batch {} with {} rows", i + 1, batch.num_rows());
        manager.write_batch_to_all_writers(&batch)?;
    }

    // Finalize all writers
    manager.finalize_all_writers()?;

    println!("✅ HLXC file created via OutputManager");
    Ok(())
}

/// Example usage of HLXC utilities
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_hlxc_round_trip() {
        let temp_dir = TempDir::new().unwrap();
        let hlxc_path = temp_dir.path().join("test.hlxc");

        // Create sample data
        let batch = create_sample_record_batch().unwrap();

        // Create HLXC file
        create_hlxc_file(&hlxc_path, vec![batch], true).unwrap();

        // Verify file exists and has content
        assert!(hlxc_path.exists());
        let metadata = std::fs::metadata(&hlxc_path).unwrap();
        assert!(metadata.len() > 0);

        // Analyze the file
        analyze_hlxc_file(&hlxc_path).unwrap();
    }

    #[test]
    fn test_compression_comparison() {
        let temp_dir = TempDir::new().unwrap();
        let json_path = temp_dir.path().join("data.json");
        let hlxc_path = temp_dir.path().join("data.hlxc");

        // Create sample data
        let batch = create_sample_record_batch().unwrap();

        // Write as JSON for comparison
        let json_data = serde_json::json!({
            "schema": batch.schema().fields().iter().map(|f| {
                serde_json::json!({
                    "name": f.name(),
                    "data_type": f.data_type().to_string(),
                    "nullable": f.is_nullable()
                })
            }).collect::<Vec<_>>(),
            "row_count": batch.num_rows()
        });
        std::fs::write(&json_path, serde_json::to_string_pretty(&json_data).unwrap()).unwrap();

        // Create HLXC file
        create_hlxc_file(&hlxc_path, vec![batch], true).unwrap();

        // Compare sizes
        compare_compression_efficiency(&json_path, &hlxc_path).unwrap();
    }
}
