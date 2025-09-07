# MSO Info Command

## Overview
The `info` command displays detailed information about compiled MSO binary files, including metadata, symbol tables, and structural analysis.

## Usage
```bash
mso info <file> [options]
```

## Arguments
- `<file>` - Path to the compiled MSO binary file (`.hlxb`)

## Options

### `-f, --format <FORMAT>`
- **Description**: Output format for information display
- **Default**: `text`
- **Formats**:
  - `text`: Human-readable text format
  - `json`: JSON format for programmatic use
  - `yaml`: YAML format (not yet implemented)
- **Example**: `mso info config.hlxb --format json`

### `--symbols`
- **Description**: Show detailed symbol table information
- **Default**: Disabled
- **Example**: `mso info config.hlxb --symbols`

### `--sections`
- **Description**: Show detailed data sections information
- **Default**: Disabled
- **Example**: `mso info config.hlxb --sections`

### `-v, --verbose`
- **Description**: Enable verbose output (includes symbols and sections)
- **Default**: Disabled
- **Example**: `mso info config.hlxb --verbose`

## Examples

### Basic Information
```bash
mso info my-config.hlxb
# Shows basic binary information
```

### Detailed Analysis
```bash
mso info my-config.hlxb --symbols --sections
# Shows comprehensive binary analysis
```

### JSON Output
```bash
mso info my-config.hlxb --format json
# Output in JSON format for scripting
```

### Verbose Information
```bash
mso info my-config.hlxb --verbose
# Shows all available information
```

## Information Displayed

### Basic Information
- **File Path**: Location of the binary file
- **Version**: MSO compiler version used
- **Platform**: Target platform information
- **Created**: Compilation timestamp
- **Optimization Level**: Optimization level used
- **Compression**: Whether compression is enabled
- **File Size**: Binary size in bytes
- **Checksum**: File integrity checksum
- **Source Path**: Original source file path (if available)

### Symbol Table (--symbols or --verbose)
- **String Statistics**:
  - Total strings count
  - Unique strings count
  - Total bytes used
- **Declaration Counts**:
  - Agents count
  - Workflows count
  - Contexts count
  - Crews count

### Data Sections (--sections or --verbose)
- **Section Information**:
  - Section type
  - Section size in bytes
  - Compression information (if applicable)
  - Section index

## Output Formats

### Text Format (Default)
Human-readable format with clear sections and labels:
```
HELIX Binary Information
=======================
File: /path/to/config.hlxb
Version: 1.0.116
Compiler: helix_config 1.0.116
Platform: x86_64-apple-darwin
Created: 2024-01-11T18:30:00Z
Optimization: Level 2
Compressed: true
Size: 12345 bytes
Checksum: a1b2c3d4
```

### JSON Format
Structured JSON output for programmatic use:
```json
{
  "file": "/path/to/config.hlxb",
  "version": "1.0.116",
  "compiler_version": "helix_config 1.0.116",
  "platform": "x86_64-apple-darwin",
  "created_at": "2024-01-11T18:30:00Z",
  "optimization_level": 2,
  "compressed": true,
  "size": 12345,
  "checksum": "a1b2c3d4"
}
```

## Error Handling
- **File Not Found**: Clear error for missing files
- **Invalid Binary**: Error for corrupted or invalid `.hlxb` files
- **Format Errors**: Issues with binary format or version
- **Permission Errors**: File system access issues

## Use Cases
- **Debugging**: Analyze compiled binary structure
- **Optimization**: Compare optimization results
- **Documentation**: Generate binary documentation
- **CI/CD**: Automated binary analysis in pipelines
- **Quality Assurance**: Verify compilation results

## Best Practices
- Use JSON format for automated analysis
- Combine with other commands for comprehensive analysis
- Use verbose mode for detailed debugging
- Compare info output before and after optimization
- Document binary characteristics for production use
