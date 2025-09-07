# HELIX Language - Rust Library Integration Examples

This directory contains Rust code examples showing how to integrate `helix_core` as a library dependency in your Rust projects.

## Examples Overview

### 1. Basic Parsing (`basic_parsing.rs`)
Demonstrates fundamental usage of the HELIX Language library:
- Parsing HELIX source code from strings
- AST validation
- Configuration conversion
- File loading
- Pretty printing

**Run with:**
```bash
cargo run --example basic_parsing
```

### 2. Compiler Integration (`compiler_integration.rs`)
Shows advanced compiler features:
- Creating optimized compilers
- Binary compilation and serialization
- Loading and decompiling binaries
- Round-trip compilation verification

**Run with:**
```bash
cargo run --example compiler_integration --features compiler
```

### 3. Advanced Usage (`advanced_usage.rs`)
Demonstrates sophisticated integration patterns:
- Complex AST analysis and manipulation
- Custom processing functions
- Advanced validation
- Usage statistics generation
- Integration with semantic analysis and code generation

**Run with:**
```bash
cargo run --example advanced_usage
```

### 4. Real Binary Loading (`load.rs`)
Shows how to load and use actual compiled `.hlxb` files:
- Loading real binary files from the `binaries/` directory
- Decompiling binaries back to AST
- Comparing different optimization levels
- Real application usage simulation
- Working with actual project files

**Run with:**
```bash
cargo run --example load --features compiler
```

## Using HELIX as a Library in Your Project

### 1. Add Dependency

Add to your `Cargo.toml`:

```toml
[dependencies]
helix = { path = "path/to/helix" }

# Or with specific features:
helix = { path = "path/to/helix", default-features = false }  # Just parsing
helix = { path = "path/to/helix", features = ["compiler"] }   # With compiler
```

### 2. Basic Usage

```rust
use helix_core::{parse, validate, load_file, ast_to_config};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse from string
    let source = "agent 'assistant' { model = 'gpt-4' }";
    let ast = parse(source)?;
    
    // Validate
    validate(&ast)?;
    
    // Convert to configuration
    let config = ast_to_config(ast)?;
    
    // Use the configuration...
    println!("Loaded {} agents", config.agents.len());
    
    Ok(())
}
```

### 3. With Compiler Features

```rust
#[cfg(feature = "compiler")]
use helix_core::compiler::{Compiler, OptimizationLevel};

#[cfg(feature = "compiler")]
fn compile_config() -> Result<(), Box<dyn std::error::Error>> {
    let compiler = Compiler::new(OptimizationLevel::Two);
    let binary = compiler.compile_file("config.hlx")?;
    
    // Use compiled binary...
    Ok(())
}
```

## Available Features

- **Default**: Includes compiler features
- **compiler**: Binary compilation, serialization, optimization
- **cli**: Command-line interface tools
- **full**: All features enabled

## API Reference

The main public API is available through:

- `helix_core::parse()` - Parse HELIX source to AST
- `helix_core::validate()` - Validate AST for correctness
- `helix_core::load_file()` - Load configuration from file
- `helix_core::ast_to_config()` - Convert AST to configuration struct
- `helix_core::compiler::Compiler` - Compiler with optimization
- `helix_core::compiler::BinarySerializer` - Binary serialization
- `helix_core::compiler::BinaryLoader` - Binary loading

## Testing the Examples

All examples include comprehensive tests:

```bash
# Run all example tests
cargo test --examples

# Run specific example tests
cargo test --example basic_parsing
cargo test --example compiler_integration --features compiler
cargo test --example advanced_usage
cargo test --example load --features compiler
```

## Integration Patterns

These examples demonstrate common integration patterns:

1. **Configuration Loading**: Parse and validate HELIX files
2. **Runtime Processing**: Use configurations in your application
3. **Binary Distribution**: Compile to optimized binaries
4. **Custom Validation**: Add domain-specific validation rules
5. **AST Manipulation**: Modify configurations programmatically
6. **Statistics and Analysis**: Analyze configuration usage

## Error Handling

All examples use proper error handling with `Result<(), Box<dyn std::error::Error>>` and demonstrate how to handle common error cases in HELIX integration.
