# MSO Watch Command

## Overview
The `watch` command monitors MSO files for changes and automatically recompiles them, providing hot reload functionality for development.

## Usage
```bash
helix watch <directory> [options]
```

## Options
- `-o, --output <directory>`: Output directory for compiled files
- `-O, --optimize <level>`: Optimization level 0-3 (default: 2)
- `--debounce <ms>`: Debounce time in milliseconds (default: 500)
- `--ignore <pattern>`: Ignore patterns (can be specified multiple times)
- `--include <pattern>`: Include patterns (can be specified multiple times)
- `--verbose, -v`: Enable verbose output

## Features

### File Watching
- **Real-time Monitoring**: Monitors file system changes using efficient native APIs
- **Pattern Filtering**: Include/exclude files using glob patterns
- **Debouncing**: Prevents excessive recompilation during rapid changes
- **Recursive Watching**: Monitors subdirectories automatically

### Hot Reload
- **Automatic Compilation**: Recompiles files when changes are detected
- **Incremental Updates**: Only recompiles changed files
- **Error Handling**: Continues watching even if compilation fails
- **Status Reporting**: Shows compilation status and errors

### Development Workflow
- **Live Development**: See changes immediately without manual compilation
- **Error Feedback**: Real-time error reporting and validation
- **Performance Monitoring**: Track compilation times and performance
- **Integration**: Works with IDEs and development tools

## Examples

### Basic Watching
```bash
# Watch current directory
helix watch .

# Watch specific directory
helix watch ./src

# Watch with custom output directory
helix watch ./src --output ./dist
```

### Advanced Configuration
```bash
# Watch with optimization and custom patterns
helix watch ./src \
  --output ./dist \
  --optimize 3 \
  --include "*.hlxbb" \
  --ignore "test_*" \
  --ignore "*.tmp" \
  --debounce 1000
```

### Development Setup
```bash
# Watch for development with verbose output
helix watch ./src \
  --output ./dist \
  --optimize 1 \
  --verbose \
  --debounce 300
```

## Configuration

### Watch Patterns

#### Include Patterns
```bash
# Only watch .hlx files
helix watch . --include "*.hlxbb"

# Watch specific file types
helix watch . --include "*.hlxbb" --include "*.json"
```

#### Exclude Patterns
```bash
# Ignore test files
helix watch . --ignore "test_*"

# Ignore temporary files
helix watch . --ignore "*.tmp" --ignore "*.bak"
```

#### Combined Patterns
```bash
# Complex pattern matching
helix watch ./src \
  --include "*.hlxbb" \
  --include "config/*.json" \
  --ignore "test_*" \
  --ignore "*.tmp" \
  --ignore "node_modules/*"
```

### Debounce Configuration
```bash
# Fast response for development
helix watch . --debounce 100

# Slower response for stability
helix watch . --debounce 2000
```

## Output Management

### Output Directory Structure
```
src/
├── main.mso
├── agents/
│   └── assistant.mso
└── workflows/
    └── chat.mso

dist/ (output)
├── main.hlxb
├── agents/
│   └── assistant.hlxb
└── workflows/
    └── chat.hlxb
```

### File Mapping
- **Source**: `src/main.mso`
- **Output**: `dist/main.hlxb`
- **Preserves**: Directory structure and relative paths

## Event Handling

### File Events
- **Created**: New files are automatically detected and compiled
- **Modified**: Changed files trigger recompilation
- **Deleted**: Removed files are handled gracefully
- **Renamed**: File renames are detected and handled

### Compilation Events
- **Success**: Files compiled successfully
- **Error**: Compilation errors are reported
- **Warning**: Compilation warnings are shown
- **Skipped**: Files that don't need recompilation

## Status Reporting

### Console Output
```
👀 Watching directory: ./src
   Press Ctrl+C to stop

📝 File changed: ./src/main.mso
   ✅ Compiled to: ./dist/main.hlxb
   📊 Size: 1,234 bytes

📝 File changed: ./src/agents/assistant.mso
   ❌ Compilation failed: Syntax error at line 15
   🔍 Error: Expected '}' but found ';'
```

### Verbose Mode
```
👀 Watching directory: ./src
   Press Ctrl+C to stop
   📁 Output directory: ./dist
   ⚡ Optimization level: 2
   ⏱️  Debounce time: 500ms
   📋 Include patterns: ["*.hlxbb"]
   🚫 Exclude patterns: ["test_*", "*.tmp"]

📝 File changed: ./src/main.mso
   🔄 Compiling with optimization level 2...
   ✅ Compiled successfully: ./dist/main.hlxb
   📊 Size: 1,234 bytes (compressed: 456 bytes)
   ⏱️  Compilation time: 23ms
```

## Integration

### IDE Integration
- **VS Code**: Works with VS Code file watchers
- **IntelliJ**: Compatible with IntelliJ file monitoring
- **Vim/Emacs**: Works with any editor that saves files

### Build Systems
- **Make**: Can be integrated with Makefiles
- **CMake**: Works with CMake build systems
- **Gradle**: Compatible with Gradle builds

### CI/CD Pipelines
- **GitHub Actions**: Can be used in CI workflows
- **Jenkins**: Integrates with Jenkins pipelines
- **GitLab CI**: Works with GitLab CI/CD

## Performance

### Optimization
- **Incremental Compilation**: Only recompiles changed files
- **Parallel Processing**: Multiple files can be compiled simultaneously
- **Caching**: Compilation results are cached when possible
- **Debouncing**: Prevents excessive recompilation

### Resource Usage
- **Memory**: Minimal memory footprint for file watching
- **CPU**: Low CPU usage during idle periods
- **Disk I/O**: Efficient file system monitoring
- **Network**: No network usage (local operation only)

## Error Handling

### Common Errors
- **Permission Denied**: Cannot access watched directory
- **Directory Not Found**: Specified directory doesn't exist
- **Compilation Errors**: Syntax or semantic errors in MSO files
- **Output Errors**: Cannot write to output directory

### Error Recovery
- **Continue Watching**: Errors don't stop the watch process
- **Error Reporting**: Clear error messages with file locations
- **Retry Logic**: Automatic retry for transient errors
- **Graceful Degradation**: Partial functionality when some files fail

## Best Practices

1. **Use Appropriate Debounce**: Balance responsiveness with performance
2. **Exclude Unnecessary Files**: Use ignore patterns to avoid watching irrelevant files
3. **Monitor Performance**: Watch compilation times and optimize accordingly
4. **Handle Errors Gracefully**: Don't let compilation errors stop development
5. **Use Verbose Mode**: Enable verbose mode for debugging and development
6. **Organize Output**: Use clear output directory structure

## Troubleshooting

### Common Issues
- **Files Not Detected**: Check include/exclude patterns
- **Slow Compilation**: Reduce optimization level or debounce time
- **Permission Errors**: Ensure read access to source and write access to output
- **High CPU Usage**: Increase debounce time or reduce optimization level

### Solutions
- **Check Patterns**: Verify include/exclude patterns are correct
- **Adjust Settings**: Tune debounce time and optimization level
- **Fix Permissions**: Ensure proper file and directory permissions
- **Monitor Resources**: Watch system resource usage and adjust accordingly