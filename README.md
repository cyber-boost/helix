<p align="center">
  <a href="https://github.com/cyber-boost/helix">
    <img src="https://raw.githubusercontent.com/cyber-boost/helix/refs/heads/master/media/logo.png"
         alt="Helix Logo"
         style="max-width: 80%;">
  </a>
</p>

# Helix – AI‑Native Configuration Language

[![Crates.io](https://img.shields.io/crates/v/hlx.svg)](https://crates.io/crates/hlx)
[![Docs](https://img.shields.io/badge/docs-available-brightgreen.svg)](https://docs.rs/hlx/latest/helix/all.html)

**Helix** is configuration language built for AI agents, model training,workflows, and data pipelines. It provides **native AI constructs**, **type‑safe values**, and **high‑performance binary compilation** while staying human‑readable for development. Configuration is still ugly looking but does a little bit more than the others.

---

## 🚀 Quick‑Start

```bash
# Install the CLI (full feature set)
cargo install --path . --features cli,full

# Create a demo project
hlx init demo

# Compile to the ultra‑fast binary format
hlx compile demo.hlx -O3 --format hlxb   # 44× faster loading


Rust example:
```rust
use helix::{hlx, parse, validate, load_file, ast_to_config};
use helix::ops::{ensure_calc, OperatorParser};
use helix::value::Value;
use helix::hlx::{HlxDatasetProcessor, start_default_server, start_server};
use helix::server::ServerConfig;

/// Process all .hlx files in a directory
pub async fn process_hlx_files(
    config_dir: &Path,
    _runtime_context: &HashMap<String, String>
) -> Result<(usize, Vec<String>)> {
    let hlx_files: Vec<_> = WalkDir::new(config_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_path_buf())
        .filter(|path| path.extension().map_or(false, |ext| ext == "hlx"))
        .collect();

    let mut processed_files = Vec::new();
    let mut total_lines = 0;

    for file in &hlx_files {
        if let Ok(content) = fs::read_to_string(file) {
            total_lines += content.lines().count();
            processed_files.push(file.display().to_string());
        }
    }

    Ok((total_lines, processed_files))
}

/// Get file statistics
pub fn get_file_stats(config_dir: &Path) -> Result<(usize, Vec<String>)> {
    let hlx_files: Vec<_> = WalkDir::new(config_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_path_buf())
        .filter(|path| path.extension().map_or(false, |ext| ext == "hlx"))
        .collect();

    let mut file_names = Vec::new();
    let mut total_lines = 0;

    for file in &hlx_files {
        file_names.push(file.display().to_string());
        if let Ok(content) = fs::read_to_string(file) {
            total_lines += content.lines().count();
        }
    }

    Ok((total_lines, file_names))
}
```
```

---

## 🌟 Recent Major Updates (September 2025)

- ✅ **Zero compilation errors** – the whole repo now builds cleanly. (Warnings work in progress)  
- 🌍 **Multi‑language SDKs** – native extensions for Python, JavaScript, PHP, and Ruby.  
- ⚡ **Real operators** – production AMQP plus 40+ fundamental operators.  
- 📦 **Advanced output formats** – HLX (text), HLXC (compressed, ZSTD), HLXB (binary, LZ4/GZIP) using Arrow 2.x columnar storage.  
- 🔧 **Dynamic language features** – `!VAR!` markers, `@env` operator, `~custom` sections, scientific‑notation numbers.  

---

## 📊 Performance

| Operation | Text (`.hlx`) | Binary (`.hlxb`) | Improvement |
|-----------|--------------|------------------|-------------|
| Load      | 35 ms        | **0.8 ms**       | **44× faster** |
| Parse     | required     | pre‑compiled     | — |
| Validate  | 12 ms        | 0 ms             | — |

*Measured on a 2023 MacBook Pro (M1, 16 GB RAM) with a 10 KB config file.*

| Format | Size | Compression | Typical use‑case |
|--------|------|-------------|------------------|
| **HLX** | 100 KB | none | Development & debugging |
| **HLXC** | 30 KB | ZSTD | Distribution of compiled configs |
| **HLXB** | 25 KB | LZ4/GZIP | Production runtime |

---

## Why Helix?  (Problems → Helix Solutions)

| Problem | Helix Solution |
|---------|----------------|
| **TOML** – limited structures, no arrays, no conditionals | **Rich type system** – arrays, objects, durations, references, pipelines. |
| **JSON** – no comments, everything is a string | **Human‑readable syntax** with comments, block delimiters, and native literals. |
| **YAML** – whitespace hell, type ambiguity | **Deterministic parsing** – a single lexer, explicit token types, no indentation quirks. |
| **ENV** – flat key/value, scattered across the system | **Scoped variables** (`@var`, `@env`, `!VAR!`) with runtime → OS fallback. |
| **AI config** – ad‑hoc scripts or custom JSON | **AI‑native constructs** – agents, workflows, pipelines, crews, memory tags. |
| **Performance** – parsing JSON/TOML each start‑up | **Binary compilation** (`.hlxb`) → **44× faster loading**; Arrow columnar format for analytics. |

---

## Core Language Features

### Agent definition
```There is 10+ special keword sections for more capabilities, like memory, queue, etc.
agent "senior-engineer" <
    model = "claude-3-opus"
    temperature = 0.7
    max_tokens = 100000

    capabilities [
        "rust-async"
        "system-design"
    ]

    backstory {
        15 years of systems programming
        Focus on safety and performance
    }
>
```

### Workflow with native durations
```hlx
workflow "code-review":
    step "analyze" {
        agent = "senior-engineer"
        timeout = 30m  # native duration!

        retry [
            max_attempts = 3
            delay = 30s
            backoff = "exponential"
        ]
    }

    pipeline <
        analyze -> test -> deploy
    >
;
```

### Flexible block delimiters & custom sections
```hlx
# All of these are equivalent:
project "app" { version = "1.0" }
project "app" < version = "1.0" >
project "app" [ version = "1.0" ]
project "app": version = "1.0" ;

# User‑defined sections with tilde prefix
~database {
    host = !DB_HOST!                # variable marker
    port = @env['DB_PORT']          # environment operator
}
```

---
## { } < > [ ] : ; @ ! Operators and Declorations are flexible to some extent, hope to be more bulletproof soon.
What you start, you finish with, like any other but within, you can change.
There is special keywords but the parser does a good job explaining the syntax.

section "whateverName":
    key = "value"
>
section whateverName [
    !key = value
]
sectionWhateverName: key = "value" ;

**in the hlx_test there is a lot of working examples with the binary and with the lib**
---

## 📚 Language SDKs

### Python
```python
import asyncio
from helix import parse, HelixInterpreter

async def main():
    cfg = parse('agent "assistant" { model = "gpt-4" temperature = 0.7 }')
    interpreter = HelixInterpreter()
    result = await interpreter.execute("@math.add(5, 3)")
    print(result)   # → 8

asyncio.run(main())
```

### JavaScript (Node)
```javascript
const { parse, HelixInterpreter } = require('helix');

(async () => {
  const cfg = parse(`
    workflow "pipeline" {
      timeout = 30m
    }
  `);
  const interpreter = new HelixInterpreter();
  const result = await interpreter.execute('@env["API_KEY"]');
  console.log(result);
})();
```

### PHP
```php
<?php
use Helix\Helix;

$hlx = new Helix();
$config = $hlx->parse('agent "bot" { model = "claude" }');
$result = $hlx->execute('@date.now()');
echo $result;   // e.g. 2025‑09‑25T12:34:56Z
```

### Ruby
```ruby
require 'helix'

config = Helix.parse('project "app" { version = "1.0" }')
ast    = Helix.ast(config)
result = Helix.execute('@string.uppercase("hello")')
puts result   # => "HELLO"
```

---

## ⚡ Operator System (selected examples)

| Category | Operators |
|----------|-----------|
| **Variables & Memory** | `@var.set`, `@memory.store`, `@memory.load` |
| **Environment & System** | `@env['KEY']`, `@sys.exec("cmd")` |
| **Data Manipulation** | `@json.parse`, `@array.filter`, `@string.uppercase` |
| **Math & Time** | `@math.add`, `@date.now`, `@time.duration("30m")` |
| **Crypto** | `@crypto.hash("sha256", data)`, `@crypto.encrypt` |
| **Production** | **AMQP**, **Redis**, **Kafka** (feature‑gated), **Elasticsearch**, **Service‑Mesh** (Istio, Consul, Vault) |

---

## 📦 CLI Commands

### Compilation & Validation
```bash
hlx compile config.hlx -O3 --format hlxb
hlx validate config.hlx --strict
hlx bundle ./configs/ -o bundle.hlxb
```

### Project Management
```bash
hlx init my-project
hlx build --release
hlx run --watch
hlx test --coverage
hlx fmt config.hlx --fix   # auto‑format
hlx lint config.hlx        # static analysis
hlx schema config.hlx --lang python   # SDK generation
```

### Server & Watch Mode
```bash
hlx serve --port 8080
hlx watch ./configs/ --auto-reload
```


---

## 📊 Current Implementation Status

| ✅ Completed | 🚧 In‑progress |
|--------------|----------------|
| Full lexer (scientific notation, variable markers) | Pipeline execution engine |
| Recursive‑descent parser with dynamic sections | HLXC random‑access reader |
| 40+ fundamental operators | Real Service‑Mesh operators |
| Production AMQP & Redis operators | HuggingFace streaming dataset loader |
| Arrow 2.x IPC + compression | GraphQL / OpenAPI schema export |
| Binary compilation (`.hlxb`) | Import statements & module system |
| Native SDKs for Python, JS, PHP, Ruby | IDE plugins (VS Code, Vim) |
| Unified build & test script | Template inheritance & custom validator framework |
| Comprehensive test suite (core + SDK) | Performance benchmarking suite for large datasets |

**Roadmap**  
- **Q1 2026** – pipeline engine, HLXC random‑access reader, HuggingFace streaming.  
- **Q2 2026** – Service‑Mesh real implementations, GraphQL/OpenAPI export, live config reloading.  
- **Q3 2026** – Import/module system, template inheritance, custom validation framework.  

---

## 🧪 Testing

```bash
# Core Rust tests
cargo test

# SDK‑specific tests (They are 75% complete, not launch yet)
pytest sdk/python/tests/
npm test --prefix sdk/js/
phpunit sdk/php/tests/
ruby sdk/ruby/test.rb

```

All test suites run on CI and finish with **0 compilation errors**.

---

## 📁 Project Structure

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

---


## Why Helix Configuration?

### The Problems We Solved

**TOML Problems:**
- No native support for complex structures
- Arrays of tables are a nightmare
- No conditionals or references
- Limited type system
- Can't express workflows or pipelines

**JSON Problems:**
- Verbose and unreadable
- No comments (seriously?)
- Everything is a string or number
- No duration types (30m → "30m" → parse → pray)
- Trailing comma hell

**ENV Variables Problems:**
- Everything is a string
- No structure or hierarchy
- No validation
- Scattered across shell scripts
- No version control friendly

**YAML Problems:**
- Whitespace sensitivity disasters
- Norway problem (no: false)
- Ambiguous types
- Anchors and references are cryptic
- Multi-line strings are painful

### The helix Solution

**Built for AI Configuration:**
- **Native AI constructs** - agents, workflows, pipelines, crews
- **Rich type system** - durations (30m), references ($VAR), tags (@memory.key)
- **Hierarchical structure** - Clean nesting without the pain
- **Comments** - Because documentation matters
- **Validation** - Catch errors at compile time, not runtime
- **Binary compilation** - Parse once, load instantly

---

## 🤝 Contributing

We are actively looking for help in the following areas:

* **Operator implementations** – Kafka, Service‑Mesh, GraphQL, OpenAPI.  
* **SDK polishing** – richer type hints, async ergonomics, documentation.  
* **Performance work** – micro‑benchmarks for Arrow IPC, binary loading, parallel compilation.  
* **Docs & examples** – more end‑to‑end tutorials, IDE extensions, live‑reloading guides.  

Please read **`CONTRIBUTING.md`** for the exact workflow:

1. Fork the repository.  
2. Create a feature branch (`git checkout -b feat/awesome‑thing`).  
3. Open a Pull Request against `main`.  

We use **semantic versioning**; releases are published on crates.io monthly.

---

## 📄 License

**MIT License** – see the `LICENSE` file / Legal folder.  
*“BBL – Configuration should enable, not constrain.”* – our guiding philosophy.

---

## 🔗 Links & Community

* **Documentation** – https://docs.rs/hlx/latest/helix/all.html (see `commands/` folder).  
* **Maestro.ps** – the platform Helix was built for: https://maestro.ps  and https::mlfor.ge
* **Examples** – `./hlx_test/` contains real‑world config files.  
* **Roadmap** – see the “Current Implementation Status” table above.  