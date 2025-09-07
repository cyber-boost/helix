# MSO Optimize Command

## Overview
The `optimize` command applies additional optimizations to existing compiled MSO binary files, improving performance and reducing file size without requiring recompilation from source.

## Usage
```bash
mso optimize <input> [options]
```

## Arguments
- `<input>` - Path to the MSO binary file (`.hlxb`) to optimize

## Options

### `-o, --output <PATH>`
- **Description**: Output file path for optimized binary
- **Default**: Overwrites input file
- **Example**: `mso optimize config.hlxb -o optimized.hlxb`

### `-O, --level <LEVEL>`
- **Description**: Optimization level (0-3)
- **Default**: 3 (maximum optimization)
- **Levels**:
  - `0`: No optimization
  - `1`: Basic optimization
  - `2`: Standard optimization
  - `3`: Maximum optimization (recommended)
- **Example**: `mso optimize config.hlxb -O3`

### `-v, --verbose`
- **Description**: Show detailed optimization information and statistics
- **Default**: Disabled
- **Example**: `mso optimize config.hlxb --verbose`

## Examples

### Basic Optimization
```bash
mso optimize config.hlxb
# Applies maximum optimization to config.hlxb
```

### Custom Output
```bash
mso optimize config.hlxb -o optimized.hlxb
# Creates optimized copy without overwriting original
```

### Specific Optimization Level
```bash
mso optimize config.hlxb -O2
# Applies standard optimization level
```

### Verbose Optimization
```bash
mso optimize config.hlxb --verbose
# Shows detailed optimization process and results
```

## Optimization Process

### Binary Analysis
- **Load Binary**: Reads and parses the input binary file
- **Convert to IR**: Transforms binary to intermediate representation
- **Analyze Structure**: Examines symbol tables and data sections

### Optimization Application
- **Dead Code Elimination**: Removes unused code paths
- **String Optimization**: Deduplicates and compresses strings
- **Symbol Table Optimization**: Optimizes symbol references
- **Data Section Optimization**: Compresses and reorganizes data

### Binary Regeneration
- **Serialize IR**: Converts optimized IR back to binary format
- **Write Output**: Saves optimized binary to specified location
- **Generate Statistics**: Creates optimization report

## Output Information

### Basic Output
- ✅ Success message with output file path
- Optimization completion confirmation

### Verbose Output
- **Optimization Results**:
  - Original binary size
  - Optimized binary size
  - Size reduction percentage
  - Optimization statistics
- **Detailed Report**:
  - Strings optimized
  - Symbols processed
  - Dead code removed
  - Performance improvements

## Optimization Types

### String Optimization
- **Deduplication**: Removes duplicate strings
- **Compression**: Compresses string storage
- **Reference Optimization**: Optimizes string references

### Symbol Table Optimization
- **Reference Consolidation**: Consolidates symbol references
- **Index Optimization**: Optimizes symbol indices
- **Metadata Compression**: Compresses symbol metadata

### Code Optimization
- **Dead Code Elimination**: Removes unused code paths
- **Instruction Optimization**: Optimizes instruction sequences
- **Control Flow Optimization**: Improves control flow structures

## Error Handling
- **File Not Found**: Clear error for missing input files
- **Invalid Binary**: Error for corrupted or invalid `.hlxb` files
- **Optimization Errors**: Issues during optimization process
- **Permission Errors**: File system access issues

## Use Cases
- **Performance Tuning**: Optimize binaries for better runtime performance
- **Size Reduction**: Reduce binary file sizes for distribution
- **Post-Compilation**: Apply additional optimizations after compilation
- **Legacy Optimization**: Optimize older binary files
- **Production Preparation**: Final optimization before deployment

## Best Practices
- **Backup Originals**: Keep original files when using in-place optimization
- **Test Thoroughly**: Verify optimized binaries work correctly
- **Use Appropriate Levels**: Choose optimization level based on needs
- **Monitor Results**: Use verbose mode to understand optimization effects
- **Version Control**: Track optimization changes in version control

## Performance Impact
- **Optimization Time**: Higher levels take longer but provide better results
- **Runtime Performance**: Optimized binaries typically run faster
- **File Size**: Optimization usually reduces file size
- **Memory Usage**: May reduce memory footprint during execution

## Limitations
- **Source Dependency**: Cannot optimize beyond what's possible from binary
- **Lossy Optimization**: Some optimizations may change behavior
- **Version Compatibility**: May not work with very old binary formats
- **Platform Specific**: Optimization effectiveness may vary by platform
