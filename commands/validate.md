# MSO Validate Command

## Overview
The `validate` command checks MSO files (both `.mso` source and `.hlxb` binary) for syntax errors, semantic issues, and structural integrity.

## Usage
```bash
mso validate <file> [options]
```

## Arguments
- `<file>` - Path to the MSO file to validate (`.mso` or `.hlxb`)

## Options

### `-d, --detailed`
- **Description**: Show detailed validation results and analysis
- **Default**: Disabled
- **Example**: `mso validate config.hlx --detailed`

### `-v, --verbose`
- **Description**: Enable verbose output (same as --detailed)
- **Default**: Disabled
- **Example**: `mso validate config.hlx --verbose`

## Examples

### Basic Validation
```bash
mso validate my-config.hlx
# Quick validation check
```

### Detailed Validation
```bash
mso validate my-config.hlx --detailed
# Comprehensive validation with detailed output
```

### Binary Validation
```bash
mso validate compiled.hlxb
# Validate compiled binary file
```

## Validation Types

### Source File Validation (`.mso`)
- **Syntax Check**: Validates MSO language syntax
- **Semantic Analysis**: Checks for logical errors
- **Type Checking**: Validates data types and references
- **Dependency Analysis**: Verifies agent and workflow references

### Binary File Validation (`.hlxb`)
- **Format Integrity**: Checks binary file structure
- **Version Compatibility**: Validates MSO version
- **Checksum Verification**: Ensures file integrity
- **Section Validation**: Verifies data sections

## Output Information

### Basic Validation
- ✅ Valid file confirmation
- File type identification
- Basic file information

### Detailed Validation
- **Source Files**:
  - Declaration count
  - AST structure analysis
  - Reference validation results
- **Binary Files**:
  - Version information
  - Section count
  - Checksum verification
  - Metadata analysis

## Error Types

### Syntax Errors
- Invalid MSO language constructs
- Missing required elements
- Malformed expressions

### Semantic Errors
- Undefined references
- Type mismatches
- Circular dependencies
- Invalid configurations

### Binary Errors
- Corrupted file structure
- Version incompatibility
- Checksum failures
- Missing sections

## Use Cases
- **Development**: Validate configurations during development
- **CI/CD**: Automated validation in build pipelines
- **Quality Assurance**: Ensure configuration integrity
- **Troubleshooting**: Diagnose configuration issues

## Best Practices
- Validate files before compilation
- Use detailed validation for complex configurations
- Integrate validation into development workflow
- Validate both source and compiled files
- Use validation in automated testing
