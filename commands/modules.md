# MSO Modules Command

## Overview
The `modules` command manages MSO module dependencies and provides unified module system functionality.

## Usage
```bash
helix modules <subcommand> [options]
```

## Subcommands

### `list`
List all modules in the current project.

```bash
helix modules list [--verbose]
```

**Options:**
- `--verbose, -v`: Show detailed module information including dependencies

**Example:**
```bash
helix modules list --verbose
```

### `resolve`
Resolve module dependencies and build dependency graph.

```bash
helix modules resolve [--check-circular]
```

**Options:**
- `--check-circular`: Check for circular dependencies

**Example:**
```bash
helix modules resolve --check-circular
```

### `bundle`
Bundle multiple modules into a single binary.

```bash
helix modules bundle <directory> [options]
```

**Options:**
- `-o, --output <file>`: Output bundle file (default: bundle.hlxb)
- `--include <pattern>`: Include patterns (can be specified multiple times)
- `-x, --exclude <pattern>`: Exclude patterns (can be specified multiple times)
- `--tree-shake`: Enable tree shaking to remove unused code
- `-O, --optimize <level>`: Optimization level 0-3 (default: 2)

**Example:**
```bash
helix modules bundle ./src --output my-bundle.hlxb --tree-shake -O3
```

### `info`
Show information about module dependencies.

```bash
helix modules info [--format <format>] [--graph]
```

**Options:**
- `--format <format>`: Output format (text, json, yaml) (default: text)
- `--graph`: Show dependency graph

**Example:**
```bash
helix modules info --format json --graph
```

## Features

### Unified Module System
- **Module Loading**: Load and parse MSO modules with dependency resolution
- **Dependency Graph**: Build and manage dependency relationships
- **Circular Detection**: Detect and prevent circular dependencies
- **Topological Sorting**: Determine optimal compilation order
- **Import/Export Management**: Handle module imports and exports
- **AST Merging**: Combine multiple modules into single AST

### Module Resolution
- **Relative Imports**: Support for `./` and `../` relative paths
- **Absolute Imports**: Support for `/` absolute paths from project root
- **Module Names**: Support for module name resolution
- **Search Paths**: Configurable search paths for module discovery

### Bundling
- **Tree Shaking**: Remove unused code and dependencies
- **Optimization**: Apply optimization levels during bundling
- **Pattern Matching**: Include/exclude files using glob patterns
- **Dependency Tracking**: Track and bundle all dependencies

## Examples

### Basic Module Management
```bash
# List all modules
helix modules list

# Resolve dependencies
helix modules resolve

# Show module information
helix modules info --graph
```

### Advanced Bundling
```bash
# Bundle with tree shaking and optimization
helix modules bundle ./src \
  --output production.hlxb \
  --tree-shake \
  --optimize 3 \
  --include "*.hlxbb" \
  --exclude "test_*"
```

### Dependency Analysis
```bash
# Check for circular dependencies
helix modules resolve --check-circular

# Export dependency graph
helix modules info --format json --graph > deps.json
```

## Integration

The modules system integrates with:
- **Compiler**: Automatic dependency resolution during compilation
- **Bundler**: Multi-file bundling with dependency tracking
- **Runtime**: Module loading and execution
- **Tools**: Migration and import/export utilities

## Technical Details

### Module Structure
```rust
pub struct Module {
    pub path: PathBuf,
    pub ast: HelixAst,
    pub exports: ModuleExports,
    pub imports: ModuleImports,
    pub metadata: ModuleMetadata,
}
```

### Dependency Graph
```rust
pub struct ModuleSystem {
    modules: HashMap<PathBuf, Module>,
    dependencies: HashMap<PathBuf, HashSet<PathBuf>>,
    dependents: HashMap<PathBuf, HashSet<PathBuf>>,
    resolution_order: Vec<PathBuf>,
}
```

### Import/Export Types
- **Agents**: AI agent configurations
- **Workflows**: Workflow definitions
- **Contexts**: Context configurations
- **Crews**: Crew definitions
- **Types**: Custom type definitions

## Error Handling

Common errors and solutions:
- **Circular Dependencies**: Use `--check-circular` to identify and resolve
- **Missing Modules**: Check search paths and file existence
- **Import Errors**: Verify import syntax and module structure
- **Resolution Failures**: Check dependency graph and compilation order

## Best Practices

1. **Organize Modules**: Use clear directory structure for modules
2. **Avoid Circular Dependencies**: Design module hierarchy carefully
3. **Use Tree Shaking**: Enable for production bundles to reduce size
4. **Optimize Dependencies**: Use appropriate optimization levels
5. **Document Exports**: Clearly document what each module exports
