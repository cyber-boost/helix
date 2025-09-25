# Helix File Types Usage Guide

This guide demonstrates how to work with all Helix file types and their dedicated utilities.

## File Types Overview

| Extension | Purpose | Compression | Human Readable | Primary Use Case |
|-----------|---------|-------------|----------------|------------------|
| `.hlx` | Text Configuration | None | ✅ | Development, version control |
| `.hlxb` | Binary Configuration | Optional | ❌ | Production, performance |
| `.hlxc` | Compressed Data | ZSTD | Partial (preview) | Large datasets, analytics |
| `.hlxj` | JSON Data | None | ✅ | Small datasets, debugging |

## Working with .hlx Files

### Loading HLX Configuration

```rust
use helix::{parse, validate, ast_to_config};

// Load from file
let content = std::fs::read_to_string("config.hlx")?;
let ast = parse(&content)?;
validate(&ast)?;
let config = ast_to_config(ast)?;
```

### Using HLX Utilities

```rust
use hlx_test::util::hlx_util::*;

// Load and validate
let config = load_hlx_config("config.hlx")?;

// Export back to HLX
let hlx_content = export_to_hlx(&config)?;

// Validate configuration
validate_hlx_config(&config)?;
```

## Working with .hlxb Files

### Creating HLXB Files

```rust
use helix::output::hlxb_config_format::{HlxbWriter, HLXB_MAGIC, HLXB_VERSION};
use std::fs::File;

let config = create_sample_config();
let file = File::create("config.hlxb")?;
let mut writer = HlxbWriter::new(file);

writer.write_header()?;
for (name, agent) in &config.agents {
    writer.write_agent(name, agent)?;
}
writer.finalize()?;
```

### Reading HLXB Files

```rust
use helix::output::hlxb_config_format::HlxbReader;

let file = File::open("config.hlxb")?;
let mut reader = HlxbReader::new(file);

let header = reader.read_header()?;
println!("Version: {}", header.version);

let config = reader.read_config()?;
```

## Working with .hlxc Files

### Creating Compressed Data Files

```rust
use helix::output::hlxc_format::HlxcWriter;
use arrow::record_batch::RecordBatch;

let record_batch = create_sample_record_batch();
let file = File::create("data.hlxc")?;

let mut writer = HlxcWriter::new(file)
    .with_compression(true)
    .with_preview(true, 5);

writer.add_batch(record_batch)?;
writer.finalize()?;
```

### Reading HLXC Files

```rust
use helix::output::hlxc_format::HlxcReader;

let file = File::open("data.hlxc")?;
let mut reader = HlxcReader::new(file);

let header = reader.read_header()?;
let preview = reader.get_preview()?;
```

## Working with .hlxj Files

### Creating Data Files

```rust
let data = serde_json::json!({
    "metadata": {
        "version": "1.0",
        "record_count": 100
    },
    "data": [
        // Your data here
    ]
});

std::fs::write("data.hlxj", serde_json::to_string_pretty(&data)?)?;
```

### Processing with GenericJSONDataset

```rust
use helix::{GenericJSONDataset, DataFormat};

let mut dataset = GenericJSONDataset::new(
    &[PathBuf::from("data.hlxj")],
    None,
    DataFormat::Auto,
)?;

// Add data
dataset.data.push(serde_json::json!({"field": "value"}));

// Detect format and convert
let training_format = dataset.detect_training_format()?;
let training_dataset = dataset.to_training_dataset()?;
```

## Best Practices

### When to Use Each Format

- **Use .hlx** for:
  - Development and testing
  - Version control
  - Human-readable configuration
  - Small to medium configurations

- **Use .hlxb** for:
  - Production deployments
  - Large configurations
  - Performance-critical applications
  - Distribution

- **Use .hlxc** for:
  - Large tabular datasets
  - Analytics and reporting
  - Data warehousing
  - Cross-platform data exchange

- **Use .hlxj** for:
  - Small datasets
  - Debugging and inspection
  - Prototyping
  - Simple data storage

### Performance Considerations

- .hlxb files load ~2-3x faster than .hlx
- .hlxc files compress data to ~30-50% of original size
- .hlxc provides columnar access for better query performance
- .hlxj files are uncompressed but fully human-readable

### File Size Guidelines

- **< 1MB**: Use .hlx or .hlxj
- **1MB - 10MB**: Consider .hlxb or .hlxc
- **> 10MB**: Definitely use .hlxc for data, .hlxb for config

## Complete Example

See `comprehensive_example.rs` for a complete demonstration of all file types working together.

## Utility Functions

All utility functions are available in the `hlx_test/util/` directory:

- `hlx_util.rs` - HLX configuration utilities
- `hlxb_util.rs` - HLXB binary configuration utilities
- `hlxc_util.rs` - HLXC compressed data utilities

Each utility module provides functions for:
- File creation and reading
- Format validation
- Performance benchmarking
- Best practices
