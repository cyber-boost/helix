# MSO Bundle Command

## Overview
The `bundle` command combines multiple MSO files from a directory into a single optimized binary, with support for filtering, tree shaking, and advanced optimization.

## Usage
```bash
mso bundle <directory> [options]
```

## Arguments
- `<directory>` - Path to directory containing MSO files to bundle

## Options

### `-o, --output <PATH>`
- **Description**: Output bundle file path
- **Default**: `bundle.hlxb`
- **Example**: `mso bundle ./configs -o production.hlxb`

### `-i, --include <PATTERN>`
- **Description**: Include files matching pattern (can be specified multiple times)
- **Default**: Include all `.mso` files
- **Example**: `mso bundle ./configs -i "*.hlxbb" -i "special/*.hlxbb"`

### `-x, --exclude <PATTERN>`
- **Description**: Exclude files matching pattern (can be specified multiple times)
- **Default**: No exclusions
- **Example**: `mso bundle ./configs -x "test/*" -x "*.backup"`

### `--tree-shake`
- **Description**: Enable tree shaking to remove unused code
- **Default**: Disabled
- **Example**: `mso bundle ./configs --tree-shake`

### `-O, --optimize <LEVEL>`
- **Description**: Optimization level (0-3)
- **Default**: 2
- **Levels**:
  - `0`: No optimization
  - `1`: Basic optimization
  - `2`: Standard optimization (recommended)
  - `3`: Maximum optimization
- **Example**: `mso bundle ./configs -O3`

### `-v, --verbose`
- **Description**: Show detailed bundling information
- **Default**: Disabled
- **Example**: `mso bundle ./configs --verbose`

## Examples

### Basic Bundling
```bash
mso bundle ./configs
# Bundles all .hlx files into bundle.hlxb
```

### Custom Output
```bash
mso bundle ./configs -o production-bundle.hlxb
# Custom output filename
```

### With Filtering
```bash
mso bundle ./configs -i "prod/*.hlxbb" -x "test/*"
# Include only production configs, exclude tests
```

### Tree Shaking
```bash
mso bundle ./configs --tree-shake -O3
# Remove unused code with maximum optimization
```

### Verbose Bundling
```bash
mso bundle ./configs --verbose
# Show detailed bundling process
```

## Bundle Features

### File Inclusion
- Automatically discovers `.mso` files in directory
- Recursive directory scanning
- Pattern-based filtering with glob syntax
- Multiple include/exclude patterns

### Tree Shaking
- Removes unused agents and workflows
- Eliminates dead code paths
- Optimizes dependency graphs
- Reduces final bundle size

### Optimization
- Cross-file optimization
- Shared resource deduplication
- String table optimization
- Symbol table compression

## Output Information
When bundling succeeds, the command displays:
- ✅ Success message with bundle file path
- Bundle size in bytes
- Number of files bundled
- Verbose mode shows additional statistics:
  - Files processed
  - Optimization results
  - Tree shaking statistics

## Error Handling
- **Directory Not Found**: Clear error for invalid paths
- **No Files Found**: Warning when no MSO files are discovered
- **Parse Errors**: Detailed error reporting for invalid files
- **Dependency Errors**: Circular dependency detection

## Use Cases
- **Production Deployment**: Bundle all configurations for deployment
- **Library Distribution**: Package HELIX configurations as libraries
- **Microservices**: Bundle related configurations together
- **Optimization**: Create highly optimized configuration bundles

## Best Practices
- Use tree shaking for production bundles
- Exclude test and development files
- Use appropriate optimization levels
- Validate individual files before bundling
- Test bundled configurations thoroughly
