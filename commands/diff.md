# MSO Diff Command

## Overview
The `diff` command compares two compiled MSO binary files and shows the differences between them, useful for analyzing changes, optimizations, and version comparisons.

## Usage
```bash
mso diff <file1> <file2> [options]
```

## Arguments
- `<file1>` - Path to the first MSO binary file (`.hlxb`)
- `<file2>` - Path to the second MSO binary file (`.hlxb`)

## Options

### `-d, --detailed`
- **Description**: Show detailed differences between the binaries
- **Default**: Disabled
- **Example**: `mso diff old.hlxb new.hlxb --detailed`

### `-v, --verbose`
- **Description**: Enable verbose output (same as --detailed)
- **Default**: Disabled
- **Example**: `mso diff old.hlxb new.hlxb --verbose`

## Examples

### Basic Comparison
```bash
mso diff config-v1.hlxb config-v2.hlxb
# Shows basic differences between two binaries
```

### Detailed Analysis
```bash
mso diff old.hlxb new.hlxb --detailed
# Comprehensive difference analysis
```

### Optimization Comparison
```bash
mso diff unoptimized.hlxb optimized.hlxb --detailed
# Compare optimization results
```

## Comparison Analysis

### Basic Differences
- **Version Comparison**: Shows version differences
- **Size Comparison**: File size differences in bytes
- **String Count**: Changes in string table size
- **Symbol Count**: Changes in symbol table entries

### Detailed Differences (--detailed)
- **Symbol Table Changes**:
  - Added/removed agents
  - Added/removed workflows
  - Added/removed contexts
  - Added/removed crews
- **String Table Analysis**:
  - New strings added
  - Strings removed
  - String count changes
- **Metadata Changes**:
  - Compiler version differences
  - Platform differences
  - Optimization level changes
  - Compression differences

## Output Information

### Basic Output
```
Comparing binaries:
  File 1: /path/to/old.hlxb
  File 2: /path/to/new.hlxb

⚠️  Version differs: 1.0.115 vs 1.0.116
⚠️  Size differs: 12345 vs 13456 bytes
⚠️  String count differs: 100 vs 105
```

### Detailed Output
- **Symbol Changes**: Detailed breakdown of symbol table differences
- **String Analysis**: Comprehensive string table comparison
- **Metadata Comparison**: Full metadata difference analysis
- **Structural Changes**: Binary structure modifications

## Error Handling
- **File Not Found**: Clear error for missing binary files
- **Invalid Binaries**: Error for corrupted or invalid `.hlxb` files
- **Version Mismatch**: Warning for binaries from different MSO versions
- **Permission Errors**: File system access issues

## Use Cases
- **Version Control**: Compare different versions of configurations
- **Optimization Analysis**: Measure optimization effectiveness
- **Debugging**: Identify changes between working and broken binaries
- **Quality Assurance**: Verify compilation consistency
- **Migration**: Compare binaries across MSO versions

## Best Practices
- Use detailed mode for comprehensive analysis
- Compare binaries from the same source files
- Document significant differences for future reference
- Use diff results to guide optimization efforts
- Combine with other analysis commands for complete picture

## Limitations
- **Source Information**: Cannot show source code differences
- **Semantic Analysis**: Limited to structural and metadata differences
- **Performance**: Large binaries may take time to compare
- **Implementation**: Detailed diff features are marked as "not yet implemented"

## Future Enhancements
- **Source Mapping**: Show source file differences
- **Semantic Diff**: Compare logical structure differences
- **Visual Output**: Graphical diff representation
- **Export Options**: Save diff results to files
- **Integration**: Integration with version control systems
