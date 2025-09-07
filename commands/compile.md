# HELIX Compile Command

## Overview
The `compile` command transforms HELIX source files (`.hlxb`) into optimized binary format (`.hlxbb`) for efficient execution and distribution.

## Usage
```bash
helix compile <input> [options]
```

## Arguments
- `<input>` - Path to the input `.mso` file to compile

## Options

### `-o, --output <PATH>`
- **Description**: Specify the output `.hlxb` file path
- **Default**: Input filename with `.hlxb` extension
- **Example**: `helix compile config.hlx -o optimized.hlxb`

### `-c, --compress`
- **Description**: Enable compression for the output binary
- **Default**: Disabled
- **Example**: `helix compile config.hlx --compress`

### `-O, --optimize <LEVEL>`
- **Description**: Set optimization level (0-3)
- **Default**: 2
- **Levels**:
  - `0`: No optimization (fastest compilation)
  - `1`: Basic optimization
  - `2`: Standard optimization (recommended)
  - `3`: Maximum optimization (slowest compilation)
- **Example**: `helix compile config.hlx -O3`

### `--cache`
- **Description**: Enable compilation caching for faster rebuilds
- **Default**: Disabled
- **Example**: `helix compile config.hlx --cache`

### `-v, --verbose`
- **Description**: Show detailed compilation information
- **Default**: Disabled
- **Example**: `helix compile config.hlx --verbose`

## Examples

### Basic Compilation
```bash
helix compile my-config.hlx
# Output: my-config.hlxb
```

### Compressed Output
```bash
helix compile my-config.hlx --compress
# Creates compressed binary for smaller file size
```

### Maximum Optimization
```bash
helix compile my-config.hlx -O3 --compress
# Maximum optimization with compression
```

### Custom Output Path
```bash
helix compile my-config.hlx -o production.hlxb
# Custom output filename
```

### Verbose Compilation
```bash
helix compile my-config.hlx --verbose
# Shows detailed compilation statistics
```

## Output Information
When compilation succeeds, the command displays:
- ✅ Success message with output file path
- Binary size in bytes
- Verbose mode shows additional statistics:
  - String count (total and unique)
  - Agent count
  - Workflow count

## Error Handling
- **File Not Found**: Clear error if input file doesn't exist
- **Parse Errors**: Detailed syntax error reporting
- **Validation Errors**: Semantic analysis error details
- **Permission Errors**: File system access issues

## Use Cases
- **Development.*HELIX configurations for testing
- **Production**: Create optimized binaries for deployment
- **Distribution**: Generate compressed binaries for sharing
- **CI/CD**: Automated compilation in build pipelines

## Performance Tips
- Use `--cache` for repeated compilations
- Higher optimization levels take longer but produce better binaries
- Compression reduces file size but may increase load time
- Verbose mode helps identify optimization opportunities
