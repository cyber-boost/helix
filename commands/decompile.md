# HELIX Decompile Command

## Overview
The `decompile` command reverses the compilation process, converting binary HELIX files (`.hlxb`) back to human-readable source format (`.mso`).

## Usage
```bash
helix decompile <input> [options]
```

## Arguments
- `<input>` - Path to the input `.hlxb` binary file to decompile

## Options

### `-o, --output <PATH>`
- **Description**: Specify the output `.mso` file path
- **Default**: Input filename with `.mso` extension
- **Example**: `helix decompile config.hlxb -o recovered.hlx`

### `-v, --verbose`
- **Description**: Show detailed decompilation information
- **Default**: Disabled
- **Example**: `helix decompile config.hlxb --verbose`

## Examples

### Basic Decompilation
```bash
helix decompile my-config.hlxb
# Output: my-config.hlx
```

### Custom Output Path
```bash
helix decompile my-config.hlxb -o recovered.hlx
# Custom output filename
```

### Verbose Decompilation
```bash
helix decompile my-config.hlxb --verbose
# Shows detailed decompilation process
```

## Output Information
When decompilation succeeds, the command displays:
- ✅ Success message with output file path
- Verbose mode shows additional information about the decompilation process

## Error Handling
- **File Not Found**: Clear error if input binary doesn't exist
- **Invalid Binary**: Error for corrupted or invalid `.hlxb` files
- **Version Mismatch**: Warning for binaries from different MSO versions
- **Permission Errors**: File system access issues

## Use Cases
- **Recovery**: Restore source code from compiled binaries
- **Debugging**: Examine compiled configurations
- **Migration**: Convert binaries between MSO versions
- **Analysis**: Study optimization effects on source code

## Limitations
- **Loss of Comments**: Comments are not preserved in binary format
- **Formatting**: Original formatting and whitespace may be lost
- **Optimizations**: Some optimizations may not be perfectly reversible
- **Metadata**: Some compilation metadata may be lost

## Best Practices
- Keep original `.mso` files as the source of truth
- Use decompilation primarily for recovery and analysis
- Verify decompiled output against original source when possible
- Use version control to track source changes, not binaries
