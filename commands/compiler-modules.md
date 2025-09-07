# HELIX Compiler Modules System - Technical Documentation

## Overview
The `compiler/modules.rs` file implements a unified module system for MSO that consolidates dependency resolution, module loading, and bundling functionality. This system replaces the previous separate `modules.rs` and `deps.rs` implementations.

## Architecture

### Core Components

#### ModuleSystem
The main orchestrator that manages all module-related operations:

```rust
pub struct ModuleSystem {
    /// Map from file path to its module
    modules: HashMap<PathBuf, Module>,
    /// Map from file path to its dependencies
    dependencies: HashMap<PathBuf, HashSet<PathBuf>>,
    /// Map from file path to files that depend on it
    dependents: HashMap<PathBuf, HashSet<PathBuf>>,
    /// Parsed ASTs for each file
    asts: HashMap<PathBuf, HelixAst>,
    /// Resolution order for compilation
    resolution_order: Vec<PathBuf>,
    /// Module resolver for finding files
    resolver: ModuleResolver,
}
```

**Key Responsibilities:**
- Module loading and parsing
- Dependency graph construction
- Circular dependency detection
- Topological sorting for compilation order
- Import/export management
- AST merging and bundling

#### Module
Represents a single MSO module/file with its metadata:

```rust
pub struct Module {
    pub path: PathBuf,
    pub ast: HelixAst,
    pub exports: ModuleExports,
    pub imports: ModuleImports,
    pub metadata: ModuleMetadata,
}
```

**Components:**
- **path**: File system path to the module
- **ast**: Parsed Abstract Syntax Tree
- **exports**: What the module makes available to other modules
- **imports**: What the module imports from other modules
- **metadata**: Version, description, and dependency information

#### ModuleExports
Defines what a module exports to other modules:

```rust
pub struct ModuleExports {
    pub agents: HashMap<String, AgentExport>,
    pub workflows: HashMap<String, WorkflowExport>,
    pub contexts: HashMap<String, ContextExport>,
    pub crews: HashMap<String, CrewExport>,
    pub types: HashMap<String, TypeExport>,
}
```

**Export Types:**
- **Agents**: AI agent configurations
- **Workflows**: Workflow definitions
- **Contexts**: Context configurations
- **Crews**: Crew definitions
- **Types**: Custom type definitions

#### ModuleImports
Defines what a module imports from other modules:

```rust
pub struct ModuleImports {
    pub imports: Vec<ImportStatement>,
}
```

#### ImportStatement
Represents a single import statement:

```rust
pub struct ImportStatement {
    pub path: String,
    pub items: ImportItems,
    pub alias: Option<String>,
}
```

**Import Types:**
- **All**: `import * from "path"`
- **Specific**: `import { agent1, workflow2 } from "path"`
- **Module**: `import "path" as alias`

### Module Resolution

#### ModuleResolver
Handles finding and resolving module files:

```rust
pub struct ModuleResolver {
    search_paths: Vec<PathBuf>,
    cache: HashMap<String, PathBuf>,
}
```

**Resolution Strategy:**
1. **Cache Check**: Check if module is already resolved
2. **Pattern Matching**: Try different file patterns
3. **Search Paths**: Search in configured directories
4. **Error Handling**: Return clear error messages

**Search Patterns:**
- `{name}.mso`
- `{name}/mod.mso`
- `mso/{name}.mso`

#### Path Resolution
Supports multiple import path types:

```rust
fn resolve_import_path(&self, import_path: &str, from_module: &Path) -> Result<PathBuf, HelixError>
```

**Path Types:**
- **Relative**: `./` and `../` relative to importing module
- **Absolute**: `/` from project root
- **Module Names**: Resolved through search paths

### Dependency Management

#### Dependency Graph
Maintains bidirectional dependency relationships:

```rust
dependencies: HashMap<PathBuf, HashSet<PathBuf>>,  // module -> its dependencies
dependents: HashMap<PathBuf, HashSet<PathBuf>>,    // module -> modules that depend on it
```

**Benefits:**
- **Efficient Lookups**: O(1) dependency and dependent lookups
- **Bidirectional Traversal**: Can traverse in both directions
- **Impact Analysis**: Determine what modules are affected by changes

#### Circular Dependency Detection
Uses depth-first search with recursion stack:

```rust
fn find_circular_dependency(&self) -> Option<Vec<PathBuf>>
fn has_cycle_util(&self, node: &PathBuf, visited: &mut HashSet<PathBuf>, 
                  rec_stack: &mut HashSet<PathBuf>, path: &mut Vec<PathBuf>) -> bool
```

**Algorithm:**
1. **DFS Traversal**: Visit each node in the dependency graph
2. **Recursion Stack**: Track current path to detect cycles
3. **Cycle Detection**: Return cycle path when detected
4. **Cleanup**: Remove nodes from recursion stack after processing

#### Topological Sorting
Determines optimal compilation order using Kahn's algorithm:

```rust
fn topological_sort(&self) -> Result<Vec<PathBuf>, HelixError>
```

**Algorithm:**
1. **In-degree Calculation**: Count incoming dependencies for each node
2. **Queue Initialization**: Start with nodes having no dependencies
3. **Processing**: Remove nodes and update in-degrees
4. **Validation**: Ensure all nodes are processed (no cycles)

### AST Operations

#### Export Extraction
Analyzes AST to determine what a module exports:

```rust
fn extract_exports(&self, ast: &HelixAst) -> Result<ModuleExports, HelixError>
```

**Process:**
1. **AST Traversal**: Walk through all declarations
2. **Type Detection**: Identify agent, workflow, context, crew declarations
3. **Export Creation**: Create export entries with metadata
4. **Location Tracking**: Record declaration locations for debugging

#### Import Extraction
Extracts import statements from AST (future feature):

```rust
fn extract_imports(&self, _ast: &HelixAst) -> Result<ModuleImports, HelixError>
```

**Current State**: Placeholder for future import statement support
**Future Implementation**: Will parse import declarations from AST

#### AST Merging
Combines multiple module ASTs into a single AST:

```rust
fn merge_modules(&self) -> Result<HelixAst, HelixError>
```

**Process:**
1. **Dependency Order**: Process modules in topological order
2. **Declaration Merging**: Combine declarations from all modules
3. **Duplicate Detection**: Avoid duplicate declarations
4. **Conflict Resolution**: Handle naming conflicts

**Duplicate Detection:**
```rust
fn declaration_exists(declarations: &[Declaration], decl: &Declaration) -> bool
```

Compares declarations by name and type to prevent duplicates.

### Bundling System

#### DependencyBundler
High-level interface for bundling multiple modules:

```rust
pub struct DependencyBundler {
    module_system: ModuleSystem,
}
```

**Features:**
- **Root File Addition**: Add entry point and resolve all dependencies
- **Dependency Resolution**: Automatically discover and load dependencies
- **Bundle Creation**: Generate single binary from multiple modules
- **Compilation Order**: Get optimal compilation order

#### Bundle Creation Process
```rust
pub fn build_bundle(&mut self) -> Result<HelixAst>
```

**Steps:**
1. **Dependency Resolution**: Resolve all module dependencies
2. **Circular Check**: Verify no circular dependencies exist
3. **AST Merging**: Combine all module ASTs
4. **Validation**: Ensure merged AST is valid

## Performance Characteristics

### Time Complexity
- **Module Loading**: O(n) where n is number of modules
- **Dependency Resolution**: O(n + m) where n is modules, m is dependencies
- **Circular Detection**: O(n + m) using DFS
- **Topological Sort**: O(n + m) using Kahn's algorithm
- **AST Merging**: O(n * d) where d is average declarations per module

### Space Complexity
- **Module Storage**: O(n) for n modules
- **Dependency Graph**: O(n + m) for n modules and m dependencies
- **AST Storage**: O(n * d) for n modules with d declarations each

### Memory Management
- **Efficient Storage**: Uses HashMaps for O(1) lookups
- **Shared References**: ASTs are shared where possible
- **Lazy Loading**: Modules loaded only when needed
- **Cache Management**: Module resolver includes caching

## Error Handling

### Error Types
- **Parse Errors**: Invalid MSO syntax in modules
- **Resolution Errors**: Cannot find imported modules
- **Circular Dependencies**: Dependency cycles detected
- **Validation Errors**: Invalid module structure

### Error Recovery
- **Graceful Degradation**: Continue processing other modules
- **Detailed Messages**: Clear error messages with file locations
- **Context Information**: Include dependency chain in errors
- **Recovery Suggestions**: Provide hints for fixing errors

## Integration Points

### Compiler Integration
- **Compilation Pipeline**: Integrates with main compiler
- **Optimization**: Works with optimization levels
- **Binary Generation**: Produces optimized binaries

### Tool Integration
- **Migration Tools**: Works with format conversion tools
- **Validation**: Integrates with validation systems
- **Debugging**: Provides debugging information

### Runtime Integration
- **Module Loading**: Supports runtime module loading
- **Hot Reload**: Enables hot reload functionality
- **Dynamic Imports**: Supports dynamic module loading

## Future Enhancements

### Planned Features
- **Import Statement Support**: Full import/export syntax
- **Module Aliasing**: Support for module aliases
- **Conditional Imports**: Conditional module loading
- **Dynamic Imports**: Runtime module loading
- **Module Caching**: Persistent module cache
- **Incremental Compilation**: Only recompile changed modules

### Performance Improvements
- **Parallel Processing**: Parallel module loading and processing
- **Incremental Updates**: Only update changed parts of dependency graph
- **Memory Optimization**: Reduce memory usage for large projects
- **Cache Optimization**: Improve cache hit rates

## Testing Strategy

### Unit Tests
- **Module Loading**: Test individual module loading
- **Dependency Resolution**: Test dependency graph construction
- **Circular Detection**: Test cycle detection algorithms
- **Topological Sort**: Test compilation order generation

### Integration Tests
- **End-to-End Bundling**: Test complete bundling process
- **Error Handling**: Test error scenarios and recovery
- **Performance Tests**: Test with large module sets
- **Compatibility Tests**: Test with various MSO file formats

### Test Data
- **Sample Modules**: Various MSO module configurations
- **Dependency Graphs**: Different dependency patterns
- **Error Cases**: Invalid modules and dependency cycles
- **Performance Data**: Large module sets for performance testing
