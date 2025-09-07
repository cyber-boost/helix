# MSO CLI Commands Documentation

This directory contains comprehensive documentation for all MSO CLI commands. Each command is documented in its own file with detailed usage information, examples, and best practices.

## Available Commands

### Core Compilation Commands

#### [Compile](compile.md)
Transform MSO source files (`.mso`) into optimized binary format (`.hlxb`)
- **Usage**: `helix compile <input> [options]`
- **Key Features**: Optimization levels, compression, caching
- **Use Cases**: Development, production deployment, distribution

#### [Decompile](decompile.md)
Convert binary HELIX files (`.hlxb`) back to human-readable source format (`.mso`)
- **Usage**: `helix decompile <input> [options]`
- **Key Features**: Source recovery, debugging support
- **Use Cases**: Recovery, debugging, migration

#### [Validate](validate.md)
Check MSO files for syntax errors, semantic issues, and structural integrity
- **Usage**: `mso validate <file> [options]`
- **Key Features**: Syntax checking, semantic analysis, binary validation
- **Use Cases**: Quality assurance, CI/CD, troubleshooting

### Advanced Operations

#### [Bundle](bundle.md)
Combine multiple MSO files into a single optimized binary
- **Usage**: `mso bundle <directory> [options]`
- **Key Features**: Tree shaking, filtering, cross-file optimization
- **Use Cases**: Production deployment, library distribution, microservices

#### [Optimize](optimize.md)
Apply additional optimizations to existing compiled MSO binaries
- **Usage**: `mso optimize <input> [options]`
- **Key Features**: Dead code elimination, string optimization, performance tuning
- **Use Cases**: Performance tuning, size reduction, post-compilation optimization

### Analysis and Comparison

#### [Info](info.md)
Display detailed information about compiled MSO binary files
- **Usage**: `mso info <file> [options]`
- **Key Features**: Metadata analysis, symbol tables, multiple output formats
- **Use Cases**: Debugging, optimization analysis, documentation

#### [Diff](diff.md)
Compare two compiled MSO binary files and show differences
- **Usage**: `mso diff <file1> <file2> [options]`
- **Key Features**: Version comparison, structural analysis, detailed differences
- **Use Cases**: Version control, optimization analysis, debugging

### Development Tools

#### [Watch](watch.md)
Monitor directory for changes and automatically recompile MSO files
- **Usage**: `mso watch <directory> [options]`
- **Key Features**: File monitoring, automatic recompilation, development workflow
- **Use Cases**: Development, testing, continuous compilation

#### [Init](init.md)
Initialize new MSO projects with embedded templates
- **Usage**: `mso init [options]`
- **Key Features**: Template selection, project scaffolding, quick start
- **Use Cases**: Project initialization, learning, prototyping

### System Management

#### [Install](install.md)
Globally install MSO compiler to system
- **Usage**: `mso install [options]`
- **Key Features**: Global installation, symlink creation, baton directory structure
- **Use Cases**: Development setup, system administration, CI/CD

## Global Options

All commands support these global options:

- `-v, --verbose`: Enable verbose output with detailed information
- `-h, --help`: Show command help and usage information
- `-V, --version`: Show MSO compiler version

## Command Categories

### **Compilation Pipeline**
1. `init` - Create new project from template
2. `validate` - Check source file integrity
3. `compile` - Transform source to binary
4. `optimize` - Apply additional optimizations
5. `bundle` - Combine multiple files

### **Analysis and Debugging**
1. `info` - Inspect binary metadata
2. `diff` - Compare binary versions
3. `decompile` - Recover source from binary
4. `validate` - Check binary integrity

### **Development Workflow**
1. `init` - Bootstrap new project
2. `watch` - Monitor for changes
3. `compile` - Build project
4. `validate` - Ensure quality

### **System Management**
1. `install` - Global installation
2. `info` - System information
3. `validate` - System health checks

## Getting Started

### Quick Start
```bash
# Install MSO globally
mso install

# Create new project
mso init --template minimal

# Compile project
helix compile minimal.mso

# Validate result
mso validate minimal.hlxb
```

### Development Workflow
```bash
# Initialize project
mso init --template ai-dev --name my-project.mso

# Watch for changes (in separate terminal)
mso watch ./src

# Compile with optimization
helix compile my-project.mso -O3 --compress

# Analyze binary
mso info my-project.hlxb --verbose
```

### Production Deployment
```bash
# Bundle multiple configurations
mso bundle ./configs --tree-shake -O3 -o production.hlxb

# Optimize final binary
mso optimize production.hlxb -O3

# Validate deployment
mso validate production.hlxb --detailed
```

## Best Practices

1. **Start Simple**: Use `init` with minimal template for learning
2. **Validate Early**: Use `validate` frequently during development
3. **Optimize Gradually**: Start with lower optimization levels
4. **Use Templates**: Leverage `init` templates for common patterns
5. **Monitor Changes**: Use `watch` for active development
6. **Analyze Results**: Use `info` and `diff` to understand optimizations
7. **Bundle for Production**: Use `bundle` for deployment packages

## Support

For additional help:
- Use `mso <command> --help` for command-specific help
- Check individual command documentation files
- Review examples in each command's documentation
- Use `--verbose` flag for detailed output and debugging
