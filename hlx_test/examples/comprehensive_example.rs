// Comprehensive Helix File Types Example
// Demonstrates working with all Helix file types: .hlx, .hlxb, .hlxc, .helix

use helix::{
    parse, validate, ast_to_config, HelixConfig,
    output::{
        hlxc_format::{HlxcWriter, HlxcReader},
        hlxb_config_format::{HlxbWriter, HlxbReader},
    },
    DataFormat, GenericJSONDataset,
};
use arrow::datatypes::*;
use arrow::record_batch::RecordBatch;
use arrow::array::*;
use std::fs;
use std::path::Path;

/// Comprehensive example showing all Helix file types
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Helix File Types Comprehensive Example");
    println!("==========================================");

    // 1. Load HLX configuration
    println!("\n1️⃣ Loading HLX Configuration");
    let hlx_path = "hlx_test/config/agent_config.hlx";
    let hlx_content = fs::read_to_string(hlx_path)?;
    println!("📄 Read HLX file ({} bytes)", hlx_content.len());

    let ast = parse(&hlx_content)?;
    validate(&ast)?;
    let config = ast_to_config(ast)?;
    println!("✅ Parsed and validated HLX config");
    println!("   - Agents: {}", config.agents.len());
    println!("   - Workflows: {}", config.workflows.len());

    // 2. Convert to HLXB binary format
    println!("\n2️⃣ Converting to HLXB Binary Format");
    let hlxb_path = "hlx_test/examples/sample_config.hlxb";
    let hlxb_file = fs::File::create(&hlxb_path)?;
    let mut hlxb_writer = HlxbWriter::new(hlxb_file);

    hlxb_writer.write_header()?;
    for (name, agent) in &config.agents {
        hlxb_writer.write_agent(name, agent)?;
    }
    for (name, workflow) in &config.workflows {
        hlxb_writer.write_workflow(name, workflow)?;
    }
    hlxb_writer.finalize()?;
    println!("✅ Created HLXB binary config");

    // 3. Read back HLXB file
    println!("\n3️⃣ Reading HLXB Binary Format");
    let hlxb_file = fs::File::open(&hlxb_path)?;
    let mut hlxb_reader = HlxbReader::new(hlxb_file);
    let header = hlxb_reader.read_header()?;
    println!("📋 HLXB Header:");
    println!("   - Magic: {:?}", std::str::from_utf8(&header.magic).unwrap_or("???"));
    println!("   - Version: {}", header.version);
    println!("   - Sections: {}", header.section_count);

    // 4. Create sample data for HLXC
    println!("\n4️⃣ Creating Sample Data for HLXC");
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, false),
        Field::new("value", DataType::Float64, true),
        Field::new("active", DataType::Boolean, false),
    ]));

    let ids = Int64Array::from(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let names = StringArray::from(vec![
        "Alice", "Bob", "Charlie", "Diana", "Eve",
        "Frank", "Grace", "Henry", "Ivy", "Jack"
    ]);
    let values = Float64Array::from(vec![
        Some(100.5), Some(250.0), None, Some(75.25), Some(300.0),
        Some(150.75), Some(200.0), None, Some(125.5), Some(350.0)
    ]);
    let active = BooleanArray::from(vec![true, true, false, true, false, true, false, true, true, false]);

    let record_batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(ids),
            Arc::new(names),
            Arc::new(values),
            Arc::new(active),
        ],
    )?;
    println!("✅ Created record batch with {} rows", record_batch.num_rows());

    // 5. Create HLXC compressed file
    println!("\n5️⃣ Creating HLXC Compressed Data File");
    let hlxc_path = "hlx_test/examples/sample_data.hlxc";
    let hlxc_file = fs::File::create(&hlxc_path)?;
    let mut hlxc_writer = HlxcWriter::new(hlxc_file)
        .with_compression(true)
        .with_preview(true, 3);

    hlxc_writer.add_batch(record_batch)?;
    hlxc_writer.finalize()?;
    println!("✅ Created compressed HLXC file");

    // 6. Read HLXC file and analyze
    println!("\n6️⃣ Analyzing HLXC File");
    let hlxc_file = fs::File::open(&hlxc_path)?;
    let mut hlxc_reader = HlxcReader::new(hlxc_file);

    let header = hlxc_reader.read_header()?;
    println!("📊 HLXC Analysis:");
    println!("   - Fields: {}", header.fields.len());
    println!("   - Compression: {}", if header.flags & 0x01 != 0 { "Enabled" } else { "Disabled" });

    if let Some(preview) = hlxc_reader.get_preview()? {
        println!("   - Preview rows: {}", preview.len());
    }

    // 7. Create .hlxj data file
    println!("\n7️⃣ Creating .hlxj Data File");
    let helix_path = "hlx_test/examples/sample.hlxj";

    // Convert record batch to JSON for .hlxj format
    let mut json_data = Vec::new();
    for row_idx in 0..record_batch.num_rows() {
        let mut row_obj = serde_json::Map::new();

        for (col_idx, field) in record_batch.schema().fields().iter().enumerate() {
            let column = record_batch.column(col_idx);

            match field.data_type() {
                DataType::Int64 => {
                    if let Some(array) = column.as_any().downcast_ref::<Int64Array>() {
                        if array.is_valid(row_idx) {
                            row_obj.insert(field.name().clone(), array.value(row_idx).into());
                        } else {
                            row_obj.insert(field.name().clone(), serde_json::Value::Null);
                        }
                    }
                }
                DataType::Utf8 => {
                    if let Some(array) = column.as_any().downcast_ref::<StringArray>() {
                        if array.is_valid(row_idx) {
                            row_obj.insert(field.name().clone(), array.value(row_idx).into());
                        } else {
                            row_obj.insert(field.name().clone(), serde_json::Value::Null);
                        }
                    }
                }
                DataType::Float64 => {
                    if let Some(array) = column.as_any().downcast_ref::<Float64Array>() {
                        if array.is_valid(row_idx) {
                            row_obj.insert(field.name().clone(), array.value(row_idx).into());
                        } else {
                            row_obj.insert(field.name().clone(), serde_json::Value::Null);
                        }
                    }
                }
                DataType::Boolean => {
                    if let Some(array) = column.as_any().downcast_ref::<BooleanArray>() {
                        if array.is_valid(row_idx) {
                            row_obj.insert(field.name().clone(), array.value(row_idx).into());
                        } else {
                            row_obj.insert(field.name().clone(), serde_json::Value::Null);
                        }
                    }
                }
                _ => {}
            }
        }

        json_data.push(serde_json::Value::Object(row_obj));
    }

    let helix_content = serde_json::json!({
        "metadata": {
            "version": "1.0",
            "created": "2024-01-15T12:00:00Z",
            "source": "comprehensive_example",
            "record_count": json_data.len()
        },
        "data": json_data
    });

    fs::write(helix_path, serde_json::to_string_pretty(&helix_content)?)?;
    println!("✅ Created .helix data file");

    // 8. File size comparison
    println!("\n8️⃣ File Size Comparison");
    let hlx_size = fs::metadata(hlx_path)?.len();
    let hlxb_size = fs::metadata(hlxb_path)?.len();
    let hlxc_size = fs::metadata(hlxc_path)?.len();
    let helix_size = fs::metadata(helix_path)?.len();

    println!("📏 File Sizes:");
    println!("   - .hlx (config): {} bytes", hlx_size);
    println!("   - .hlxb (binary config): {} bytes", hlxb_size);
    println!("   - .hlxc (compressed data): {} bytes", hlxc_size);
    println!("   - .helix (data): {} bytes", helix_size);

    // 9. Demonstrate data processing with GenericJSONDataset
    println!("\n9️⃣ Data Processing Example");
    let dataset = GenericJSONDataset::new(
        &[std::path::PathBuf::from(helix_path)],
        None,
        DataFormat::Auto,
    )?;

    // Add sample data
    let mut sample_data = Vec::new();
    for i in 1..=3 {
        sample_data.push(serde_json::json!({
            "prompt": format!("Sample prompt {}", i),
            "chosen": format!("Chosen response {}", i),
            "rejected": format!("Rejected response {}", i)
        }));
    }
    dataset.data.extend(sample_data);

    let training_format = dataset.detect_training_format()?;
    println!("🎯 Detected training format: {:?}", training_format);

    let training_dataset = dataset.to_training_dataset()?;
    println!("📚 Created training dataset with {} samples", training_dataset.samples.len());

    // 10. Summary
    println!("\n🎉 Comprehensive Example Complete!");
    println!("==========================================");
    println!("✅ Successfully demonstrated all Helix file types:");
    println!("   - .hlx: Text configuration files");
    println!("   - .hlxb: Binary configuration files");
    println!("   - .hlxc: Compressed columnar data files");
    println!("   - .helix: General-purpose data files");
    println!("");
    println!("📁 Generated files:");
    println!("   - {}", hlxb_path);
    println!("   - {}", hlxc_path);
    println!("   - {}", helix_path);

    Ok(())
}
