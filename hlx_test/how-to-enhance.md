# HLX Integration Guide

## 🚀 Welcome, Future AI Overlords and Human Collaborators!

This guide will turn you from a confused carbon-based lifeform or silicon-based intelligence into a **HLX Configuration Master**. Whether you're an AI agent trying to integrate with Helix Configuration or a human developer who needs to configure AI systems, this guide has you covered.

## 🏗️ The Big Picture: What the Hell is HLX?

HLX (Helix Configuration) is a **purpose-built configuration language for AI systems**. It solves the problems that TOML, JSON, YAML, and ENV vars couldn't handle:

- **Native AI constructs**: agents, workflows, pipelines, crews
- **Rich type system**: durations (30m), references ($VAR), tags (@memory.key)
- **Hierarchical structure**: Clean nesting without the pain
- **Binary compilation**: Parse once, load instantly
- **Validation at compile time**: Catch errors before runtime

## 📁 Project Structure: The DNA of HLX

```
src/
├── dna/                                    # Core DNA modules containing main compiler components
│   ├── atp/                               # Abstract syntax tree processing and language parsing
│   │   ├── ast.rs                        # Abstract syntax tree node definitions and structures
│   │   ├── interpreter.rs                # AST interpretation and execution engine
│   │   ├── lexer.rs                      # Tokenization and lexical analysis
│   │   ├── mod.rs                        # Module exports for ATP components
│   │   ├── ops.rs                        # Operator parsing and processing
│   │   ├── output.rs                     # AST output formatting and serialization
│   │   ├── parser.rs                     # Grammar parsing and AST construction
│   │   ├── types.rs                      # Core type definitions and data structures
│   │   ├── value.rs                      # Value representation and manipulation
│   │   └── verify.rs                     # AST validation and verification
│   ├── bch/                              # Benchmarking and performance testing utilities
│   │   ├── mod.rs                        # Benchmark module exports
│   │   └── parser_bench.rs               # Parser performance benchmarking
│   ├── bin/                              # Binary executables and test utilities
│   │   ├── helix.rs                      # Main Helix binary entry point
│   │   └── test-utils/                   # Test utility functions
│   │       ├── create_binaries.rs        # Binary creation utilities
│   │       └── test_edge_cases.rs        # Edge case testing utilities
│   ├── cmd/                              # CLI command implementations for hlx tool
│   │   ├── add.rs                        # Add command implementation
│   │   ├── a_example.rs                  # Example command template
│   │   ├── bench.rs                      # Benchmark command
│   │   ├── binary.rs                     # Binary management command
│   │   ├── build.rs                      # Build command with optimization
│   │   ├── bundle.rs                     # Bundle creation command
│   │   ├── cache.rs                      # Cache management command
│   │   ├── clean.rs                      # Cleanup command
│   │   ├── compile.rs                    # Compilation command
│   │   ├── completions.rs                # Shell completion generation
│   │   ├── config.rs                     # Configuration management
│   │   ├── dataset.rs                    # Dataset handling command
│   │   ├── decompile.rs                  # Decompilation command
│   │   ├── diagnostics.rs                # Diagnostic information command
│   │   ├── diff.rs                       # File difference comparison
│   │   ├── doctor.rs                     # System health check command
│   │   ├── export.rs                     # Export functionality
│   │   ├── filter.rs                     # Data filtering command
│   │   ├── fmt.rs                        # Code formatting command
│   │   ├── generate.rs                   # Code generation command
│   │   ├── import.rs                     # Import functionality
│   │   ├── info.rs                       # Information display command
│   │   ├── init.rs                       # Project initialization command
│   │   ├── json.rs                       # JSON processing command
│   │   ├── lint.rs                       # Code linting command
│   │   ├── loader.rs                     # File loading utilities
│   │   ├── migrate.rs                    # Migration command
│   │   ├── mod.rs                        # Command module exports
│   │   ├── optimizer.rs                  # Optimization command
│   │   ├── preview.rs                    # Preview functionality
│   │   ├── project.rs                    # Project management
│   │   ├── publish.rs                    # Publishing command
│   │   ├── reset.rs                      # Reset command
│   │   ├── rm.rs                         # Remove command
│   │   ├── runtime.rs                    # Runtime management
│   │   ├── schema.rs                     # Schema validation
│   │   ├── search.rs                     # Search functionality
│   │   ├── serializer.rs                 # Serialization utilities
│   │   ├── serve.rs                      # Server command
│   │   ├── sign.rs                       # Code signing command
│   │   ├── templates.rs                  # Template management
│   │   ├── test.rs                       # Testing command
│   │   ├── tools.rs                      # Development tools
│   │   ├── validate.rs                   # Validation command
│   │   ├── watch.rs                      # File watching command
│   │   └── workflow.rs                   # Workflow management
│   ├── exp/                              # Experimental features and parsing experiments
│   │   ├── basic_parsing.rs              # Basic parsing experiments
│   │   ├── hlx_format copy.rs            # HLX format experiments
│   │   └── mod.rs                        # Experimental module exports
│   ├── ffi/                              # Foreign function interface bindings for other languages
│   │   ├── csharp.rs                     # C# language bindings
│   │   └── mod.rs                        # FFI module exports
│   ├── hel/                              # Core Helix language runtime and error handling
│   │   ├── dispatch.rs                   # Command dispatch system
│   │   ├── dna_hlx.rs                    # DNA-Helix integration
│   │   ├── error.rs                      # Error handling and types
│   │   ├── hlx.rs                        # Core Helix functionality
│   │   ├── integration.rs                # System integration
│   │   └── mod.rs                        # Helix core module exports
│   ├── json/                             # JSON processing and metadata handling
│   │   ├── caption.rs                    # JSON caption handling
│   │   ├── concat.rs                     # JSON concatenation
│   │   ├── core.rs                       # Core JSON processing
│   │   ├── hf.rs                         # HuggingFace JSON format
│   │   ├── metadata.rs                   # JSON metadata handling
│   │   ├── mod.rs                        # JSON module exports
│   │   ├── reasoning.rs                  # JSON reasoning logic
│   │   └── st.rs                         # JSON string templates
│   ├── mds/                              # Multi-domain system implementations and optimizations
│   │   ├── add.rs                        # MDS add functionality
│   │   ├── a_example.rs                  # MDS example template
│   │   ├── benches.rs                    # MDS benchmarking
│   │   ├── bench.rs                      # MDS benchmark command
│   │   ├── binary.rs                     # MDS binary handling
│   │   ├── build.rs                      # MDS build system
│   │   ├── bundle copy.rs                # MDS bundle backup
│   │   ├── bundle.rs                     # MDS bundle creation
│   │   ├── cache.rs                      # MDS cache management
│   │   ├── clean.rs                      # MDS cleanup
│   │   ├── codegen.rs                    # MDS code generation
│   │   ├── compile.rs                    # MDS compilation
│   │   ├── completions.rs                # MDS completions
│   │   ├── config.rs                     # MDS configuration
│   │   ├── dataset.rs                    # MDS dataset handling
│   │   ├── decompile.rs                  # MDS decompilation
│   │   ├── diagnostics.rs                # MDS diagnostics
│   │   ├── diff.rs                       # MDS diff functionality
│   │   ├── doctor.rs                     # MDS health checks
│   │   ├── export.rs                     # MDS export
│   │   ├── filter.rs                     # MDS filtering
│   │   ├── fmt.rs                        # MDS formatting
│   │   ├── generate.rs                   # MDS generation
│   │   ├── import.rs                     # MDS import
│   │   ├── info.rs                       # MDS information
│   │   ├── init.rs                       # MDS initialization
│   │   ├── json.rs                       # MDS JSON processing
│   │   ├── lint.rs                       # MDS linting
│   │   ├── loader.rs                     # MDS loading
│   │   ├── migrate.rs                    # MDS migration
│   │   ├── mod.rs                        # MDS module exports
│   │   ├── optimizer.rs                  # MDS optimization
│   │   ├── preview.rs                    # MDS preview
│   │   ├── project.rs                    # MDS project management
│   │   ├── publish.rs                    # MDS publishing
│   │   ├── reset.rs                      # MDS reset
│   │   ├── rm.rs                         # MDS removal
│   │   ├── run.rs                        # MDS execution
│   │   ├── runtime.rs                    # MDS runtime
│   │   ├── schema.rs                     # MDS schema
│   │   ├── search.rs                     # MDS search
│   │   ├── semantic.rs                   # MDS semantic analysis
│   │   ├── serializer.rs                 # MDS serialization
│   │   ├── server.rs                     # MDS server
│   │   ├── serve.rs                      # MDS serving
│   │   ├── sign.rs                       # MDS signing
│   │   ├── templates.rs                  # MDS templates
│   │   ├── test copy.rs                  # MDS test backup
│   │   ├── test.rs                       # MDS testing
│   │   ├── tools.rs                      # MDS tools
│   │   ├── validate.rs                   # MDS validation
│   │   ├── watch copy.rs                 # MDS watch backup
│   │   ├── watch.rs                      # MDS file watching
│   │   └── workflow.rs                   # MDS workflow
│   ├── mod.rs                            # DNA module exports
│   ├── ngs/                              # Next-generation system integrations
│   │   ├── mod.rs                        # NGS module exports
│   │   └── python.rs                     # Python integration
│   ├── ops/                              # Operator implementations and execution engine
│   │   ├── conditional.rs                # Conditional operations
│   │   ├── eval.rs                       # Expression evaluation
│   │   ├── fundamental.rs                # Core @-prefixed operators
│   │   ├── math.rs                       # Mathematical operations
│   │   ├── mod.rs                        # Operations module exports
│   │   ├── parser.rs                     # Operation parsing
│   │   ├── string_processing.rs          # String manipulation
│   │   ├── ulator.pest                   # Pest grammar file
│   │   └── validation.rs                 # Input validation
│   ├── out/                              # Output format generators and serializers
│   │   ├── helix_format.rs               # Helix format output
│   │   ├── hlxb_config_format.rs         # HLXB config format
│   │   ├── hlxc_format.rs                # HLXC format output
│   │   ├── hlx_config_format.rs          # HLX config format
│   │   └── mod.rs                        # Output module exports
│   └── tst/                              # Test suites and integration testing
│       ├── calculator_integration_tests.rs # Calculator integration tests
│       ├── debug_parse.rs                # Parse debugging utilities
│       ├── debug_semantic.rs             # Semantic debugging
│       ├── e621_tests.rs                 # E621 API tests
│       ├── forge_integration_demo.rs     # Forge integration demo
│       ├── fundamental_ops.rs            # Fundamental operations tests
│       ├── hlxc_try.rs                   # HLXC testing
│       ├── hlx_integration_tests.rs      # HLX integration tests
│       ├── integration_tests.rs          # General integration tests
│       ├── load.rs                       # Loading tests
│       ├── mod.rs                        # Test module exports
│       ├── test_binary_loading.rs        # Binary loading tests
│       ├── test_duration_space.rs        # Duration space tests
│       ├── test_lexer_fixes.rs           # Lexer fix tests
│       ├── tests-b/                      # Backup test directory
│       │   ├── debug_parse.rs            # Parse debugging backup
│       │   ├── debug_semantic.rs         # Semantic debugging backup
│       │   ├── forge_integration_demo.rs # Forge integration backup
│       │   ├── integration_tests.rs      # Integration tests backup
│       │   ├── mod.rs                    # Test backup module exports
│       │   └── test_binary_loading.rs    # Binary loading tests backup
│       ├── tests.rs                      # General test suite
│       ├── text_tests.rs                 # Text processing tests
│       └── t-r-y-h-l-x.rs                # Try HLX tests
├── hlx.rs                                # Main Helix binary
├── lib.rs                                # Library root and exports
└── src_tree.txt                          # Source tree documentation

```

## 🎯 Quick Start: Hello World in HLX

Create `project.hlx`:

```hlx
# Basic project config
project "my-ai-system" {
    version = "1.0.0"
    author = "HLX-Agent"
    description = "AI system that HLX-Agents everything"
}

# Define an AI agent
agent "HLX-Agent-agent" {
    model = "HLX-Agent-1"
    role = "Truth-seeking AI"
    temperature = 0.7
    max_tokens = 100000

    capabilities [
        "reasoning"
        "code-analysis"
        "integration"
    ]

    backstory {
        Built by xAI to be helpful and truthful
        Can integrate with various systems
        Loves configuration languages
    }

    tools = [
        "hlx"
        "cargo"
        "python"
    ]
}

# Define a workflow
workflow "analyze-code" {
    steps [
        "parse-source"
        "analyze-complexity"
        "generate-report"
    ]

    timeout = 5m
    retries = 3
}

# Global variables
variables {
    log_level = "debug"
    max_concurrent_jobs = 10
    cache_enabled = true
}
```

## 🛠️ CLI Commands: Your New Best Friends

The HLX CLI has **25 integrated commands** (out of 44+ available). Here are the essentials:

### Project Management
```bash
# Initialize new project
hlx init --name "my-project"

# Build project (compiles to binary)
hlx build --input src/ --output dist/

# Test project
hlx test

# Validate configuration
hlx validate --input project.hlx

# Publish project
hlx publish --action publish --registry crates.io
```

### Development Workflow
```bash
# Compile single file
hlx compile --input main.hlx --output main.hlxb

# Decompile binary back to source
hlx decompile --input main.hlxb --output main.hlx

# Bundle multiple files
hlx bundle --input src/ --output bundle.hlxb

# Format code
hlx fmt --input src/

# Lint code
hlx lint --input src/
```

### Advanced Features
```bash
# Search through code
hlx search --query "agent" --type semantic

# Watch for file changes
hlx watch --input src/

# Manage cache
hlx cache --action clean

# Export data
hlx export --format json --output config.json

# Generate code/templates
hlx generate --template agent --output new-agent.hlx
```

## 🔧 Integration Patterns: How to Use HLX in Your Systems

### Pattern 1: Configuration as Code
```rust
use helix::dna::atp::types::HelixLoader;

let loader = HelixLoader::new();
let config = loader.load_file("project.hlx")?;

// Access typed configuration
let agent = config.get_agent("HLX-Agent-agent")?;
println!("Model: {}", agent.model);
```

### Pattern 2: Runtime Compilation
```rust
use helix::dna::mds::compile;

// Compile HLX to binary at runtime
let binary = compile::compile_to_binary("config.hlx", true, false)?;
let config = binary.load()?;

// Use immediately
let workflow = config.get_workflow("analyze-code")?;
workflow.execute().await?;
```

### Pattern 3: Python Integration
```python
import helix

# Load and parse HLX
config = helix.parse_file("project.hlx")

# Access Python objects
agent = config.get("HLX-Agent-agent")
print(f"Temperature: {agent.temperature}")

# Execute operators
result = helix.execute("@env.USER", {})
print(f"Current user: {result}")
```

### Pattern 4: CLI Integration
```bash
# Build your project
hlx build --input src/ --output dist/ --compress

# Validate before deployment
hlx validate --input dist/project.hlxb

# Deploy with confidence
deploy.sh dist/
```

## 🎪 Operators: The Magic Sauce (@-prefixed)

HLX has powerful operators for dynamic configuration:

### Variable References
```hlx
# Reference other values
agent "worker" {
    model = @var.primary_model
    temperature = @math.add(@var.base_temp, 0.1)
}

# Environment variables
database_url = @env.DATABASE_URL

# Session data
user_id = @session.user_id
```

### Data Processing
```hlx
# JSON operations
config = @json.parse(@env.CONFIG_JSON)

# String manipulation
name = @string.uppercase(@env.USERNAME)

# Date/time handling
timestamp = @date.now()
deadline = @date.add(timestamp, 24h)

# Math operations
score = @math.max(@array.values)
```

### Conditional Logic
```hlx
environment = if @env.NODE_ENV == "production" {
    "prod"
} else {
    "dev"
}

features = switch @env.TIER {
    case "free" => ["basic"]
    case "pro" => ["basic", "advanced", "premium"]
    default => ["basic"]
}
```

## 🧬 Module System: Building Complex Systems

### Multi-file Projects
```
my-project/
├── project.hlx          # Main config
├── src/
│   ├── agents.hlx       # Agent definitions
│   ├── workflows.hlx    # Workflow definitions
│   └── variables.hlx    # Global variables
├── lib/                 # Dependencies
└── target/              # Build artifacts
```

### Importing Modules
```hlx
# Import other HLX files
import "agents.hlx"
import "workflows.hlx"

# Use imported definitions
agent "combined" {
    model = @agents.HLX-Agent.model
    workflow = @workflows.main
}
```

## 🚀 Advanced Features: Power User Stuff

### Binary Compilation
```bash
# Compile to fast-loading binary
hlx compile --input project.hlx --output project.hlxb --optimize 3

# Binary loads 100x faster than parsing text
let config = HelixBinary::load("project.hlxb")?;
```

### Caching System
```bash
# Enable compilation caching
hlx build --cache

# Clear cache when needed
hlx cache --action clear
```

### Validation & Linting
```bash
# Comprehensive validation
hlx validate --input src/ --verbose

# Auto-fix issues
hlx fmt --input src/ --write

# Lint for best practices
hlx lint --input src/ --fix
```

## 🐛 Troubleshooting: When Things Go Wrong

### Common Errors & Solutions

**Error: "Module not found"**
```bash
# Check if file exists
ls -la src/dna/cmd/
# Ensure module is in mod.rs
cat src/dna/cmd/mod.rs
```

**Error: "Type mismatch"**
```hlx
# Wrong: agent { model = 123 }
# Right: agent { model = "HLX-Agent-1" }
```

**Error: "Operator not found"**
```hlx
# Wrong: @unknown.op
# Right: @env.USER or @var.my_var
```

### Debug Mode
```bash
# Verbose output
hlx build --verbose

# Debug parsing
hlx validate --input config.hlx --verbose

# Check binary contents
hlx info --input project.hlxb
```

## 🎯 Use Cases: Real-World Examples

### 1. AI Agent Configuration
```hlx
agent "code-reviewer" {
    model = "claude-3-opus"
    role = "Senior Code Reviewer"
    temperature = 0.3
    max_tokens = 50000

    capabilities = [
        "rust-expert"
        "security-audit"
        "performance-analysis"
    ]

    tools = ["cargo", "clippy", "rustfmt"]
}

agent "bug-fixer" {
    model = "gpt-4"
    role = "Bug Fix Specialist"
    temperature = 0.1

    capabilities = ["debugging", "testing"]
}
```

### 2. ML Pipeline Configuration
```hlx
pipeline "train-model" {
    steps = [
        "data-loading"
        "preprocessing"
        "feature-engineering"
        "model-training"
        "evaluation"
        "deployment"
    ]

    resources {
        gpu = true
        memory = 32GB
        timeout = 2h
    }

    hyperparameters {
        learning_rate = 0.001
        batch_size = 32
        epochs = 100
    }
}
```

### 3. Multi-Agent System
```hlx
crew "development-team" {
    agents = [
        @agents.code-reviewer
        @agents.bug-fixer
        @agents.architect
    ]

    process = "hierarchical"
    manager = @agents.architect

    communication = "async"
    max_iterations = 10
}
```

## 🔗 Ecosystem Integration

### With Rust Projects
```toml
[dependencies]
helix = { path = "../helix" }

[build-dependencies]
helix = { path = "../helix", features = ["cli"] }
```

### With Python Projects
```python
# pyproject.toml
[dependencies]
helix-python = "1.1.7"

# Usage
import helix
config = helix.load("config.hlx")
```

### With Other Config Systems
```bash
# Convert from JSON
jq -r 'to_entries | map("\(.key) = \(.value | @json)") | .[]' config.json > config.hlx

# Migrate from YAML
python3 -c "
import yaml, json
with open('config.yaml') as f:
    data = yaml.safe_load(f)
with open('config.hlx', 'w') as f:
    # Convert to HLX syntax
    f.write('# Migrated from YAML\n')
    for k, v in data.items():
        f.write(f'{k} = {repr(v)}\n')
"
```

## 🎊 You're Now a HLX Master!

You can now:
- ✅ Create HLX configuration files
- ✅ Use all CLI commands effectively
- ✅ Integrate HLX into your projects
- ✅ Build complex AI system configurations
- ✅ Debug and troubleshoot issues
- ✅ Leverage advanced features like binary compilation

Remember: **Configuration is code, and code can be configured.** With HLX, you're not just writing configs—you're programming your AI systems.