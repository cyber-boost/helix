# Helix Schema Command

The `hlx schema` command generates SDKs (Software Development Kits) in multiple programming languages from Helix configuration files. This allows developers to programmatically access and manipulate Helix configurations using familiar language idioms and access patterns.

## Overview

The schema command takes a `.hlx` file and generates a complete SDK class/library that provides:
- Type-safe access to configuration data
- Multiple access patterns (dot notation, bracket notation, method calls)
- Serialization/deserialization capabilities
- Processing and compilation methods
- Cross-language consistency

## Command Syntax

```bash
hlx schema [OPTIONS] <TARGET>

# Positional arguments:
TARGET    Path to the .hlx file to process

# Options:
-l, --lang <LANG>     Target programming language [default: rust]
                      [possible values: rust, python, java-script, c-sharp, java, go, ruby, php]
-o, --output <OUTPUT> Optional custom output file path
-h, --help            Print help information
-V, --version         Print version information
```

## Supported Languages

The schema command supports generating SDKs in 8 programming languages:

| Language | Enum Value | File Extension | Description |
|----------|------------|----------------|-------------|
| Rust | `rust` | `.rs` | Memory-safe systems programming with zero-cost abstractions |
| Python | `python` | `.py` | Dynamic scripting with extensive libraries |
| JavaScript | `java-script` | `.js` | Browser and Node.js runtime support |
| C# | `c-sharp` | `.cs` | .NET framework integration |
| Java | `java` | `.java` | Enterprise JVM applications |
| Go | `go` | `.go` | Concurrent systems programming |
| Ruby | `ruby` | `.rb` | Dynamic object-oriented scripting |
| PHP | `php` | `.php` | Web development and server-side scripting |

## Usage Examples

### Basic Usage

```bash
# Generate Rust SDK (default)
hlx schema config.hlx

# Generate Python SDK
hlx schema config.hlx --lang python

# Generate JavaScript SDK with custom output
hlx schema config.hlx -l java-script -o my-config.js

# Generate C# SDK
hlx schema config.hlx --lang c-sharp --output ConfigSdk.cs
```

### File Naming Convention

When no custom output path is specified, the generated file follows this pattern:
```
{input_filename}_schema.{extension}
```

Examples:
- `config.hlx` → `config_schema.rs` (Rust)
- `config.hlx` → `config_schema.py` (Python)
- `my_app.hlx` → `my_app_schema.js` (JavaScript)

## Generated SDK API

Each generated SDK provides a consistent API across all supported languages.

### Class/Module Structure

```javascript
class HelixConfig {
    constructor()
    
    // Factory methods
    static fromFile(path)
    static fromString(content)
    
    // Access methods
    get(key)
    set(key, value)
    
    // Processing methods
    process()
    compile()
}
```

### Access Patterns

The SDKs support multiple ways to access configuration data:

#### 1. Dot Notation
```javascript
const config = HelixConfig.fromFile('config.hlx');
const value = config.database.host;
config.database.port = 5432;
```

#### 2. Bracket Notation
```javascript
const host = config['database']['host'];
config['database']['port'] = 5432;
```

#### 3. Method Access
```javascript
const host = config.get('database.host');
config.set('database.port', 5432);
```

#### 4. Direct Property Access (where supported)
```python
host = config.database.host
config.database.port = 5432
```

### Processing and Compilation

```javascript
const config = HelixConfig.fromFile('config.hlx');

// Process the configuration
config.process(); // Outputs: "Processing Helix configuration..."

// Compile to serialized format
const compiled = config.compile(); // Returns serialized data
```

## Language-Specific Details

### Rust SDK

```rust
pub struct HelixConfig {
    data: HashMap<String, serde_json::Value>,
}

impl HelixConfig {
    pub fn new() -> Self
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>>
    pub fn from_string(content: &str) -> Result<Self, Box<dyn std::error::Error>>
    pub fn get(&self, key: &str) -> Option<&serde_json::Value>
    pub fn set(&mut self, key: &str, value: serde_json::Value)
    pub fn process(&self) -> Result<(), Box<dyn std::error::Error>>
    pub fn compile(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>>
}

impl std::ops::Index<&str> for HelixConfig {
    type Output = serde_json::Value;
    fn index(&self, key: &str) -> &Self::Output;
}
```

### Python SDK

```python
class HelixConfig:
    def __init__(self)
    
    @classmethod
    def from_file(cls, path: str) -> 'HelixConfig'
    @classmethod
    def from_string(cls, content: str) -> 'HelixConfig'
    
    def get(self, key: str) -> Any
    def set(self, key: str, value: Any)
    
    def __getitem__(self, key: str) -> Any
    def __setitem__(self, key: str, value: Any)
    
    def process(self)
    def compile(self) -> bytes
```

### JavaScript SDK

```javascript
class HelixConfig {
    constructor()
    
    static fromFile(path)
    static fromString(content)
    
    get(key)
    set(key, value)
    
    process()
    compile() // Returns Buffer
}

// Bracket notation access
HelixConfig.prototype.__defineGetter__('hlx', function() {
    return this;
});

module.exports = HelixConfig;
```

### C# SDK

```csharp
public class HelixConfig
{
    private Dictionary<string, object> data = new Dictionary<string, object>();
    
    public static HelixConfig FromFile(string path)
    public static HelixConfig FromString(string content)
    
    public object Get(string key)
    public void Set(string key, object value)
    
    public object this[string key] { get; set; }
    
    public void Process()
    public byte[] Compile()
}
```

### Java SDK

```java
public class HelixConfig {
    private Map<String, Object> data = new HashMap<>();
    private static final ObjectMapper mapper = new ObjectMapper();
    
    public static HelixConfig fromFile(String path) throws IOException
    public static HelixConfig fromString(String content) throws IOException
    
    public Object get(String key)
    public void set(String key, Object value)
    
    public void process()
    public byte[] compile() throws IOException
}
```

### Go SDK

```go
package helix

type HelixConfig struct {
    Data map[string]interface{} `json:"data"`
}

func NewHelixConfig() *HelixConfig
func FromFile(path string) (*HelixConfig, error)
func FromString(content string) (*HelixConfig, error)

func (h *HelixConfig) Get(key string) interface{}
func (h *HelixConfig) Set(key string, value interface{})

func (h *HelixConfig) Process()
func (h *HelixConfig) Compile() ([]byte, error)
```

### Ruby SDK

```ruby
class HelixConfig
  attr_accessor :data
  
  def initialize
    @data = {}
  end
  
  def self.from_file(path)
    content = File.read(path)
    from_string(content)
  end
  
  def self.from_string(content)
    instance = new
    instance.data = JSON.parse(content)
    instance
  end
  
  def get(key)
    @data[key]
  end
  
  def set(key, value)
    @data[key] = value
  end
  
  def [](key)
    get(key)
  end
  
  def []=(key, value)
    set(key, value)
  end
  
  def process
    puts 'Processing Helix configuration...'
  end
  
  def compile
    puts 'Compiling Helix configuration...'
    JSON.dump(@data).bytes
  end
end
```

### PHP SDK

```php
<?php

class HelixConfig {
    private $data = [];
    
    public static function fromFile(string $path): self {
        $content = file_get_contents($path);
        return self::fromString($content);
    }
    
    public static function fromString(string $content): self {
        $instance = new self();
        $instance->data = json_decode($content, true);
        return $instance;
    }
    
    public function get(string $key) {
        return $this->data[$key] ?? null;
    }
    
    public function set(string $key, $value): void {
        $this->data[$key] = $value;
    }
    
    public function __get(string $key) {
        return $this->get($key);
    }
    
    public function __set(string $key, $value): void {
        $this->set($key, $value);
    }
    
    public function process(): void {
        echo "Processing Helix configuration...\n";
    }
    
    public function compile(): string {
        echo "Compiling Helix configuration...\n";
        return json_encode($this->data);
    }
}
```

## Error Handling

The schema command will fail if:

1. **Invalid Helix file**: The target file contains syntax errors
2. **Unsupported language**: Invalid language enum value
3. **File access issues**: Cannot read the input file or write to output location
4. **Validation failures**: The Helix configuration fails semantic validation

Error messages are descriptive and indicate the specific issue encountered.

## Integration Examples

### Using Generated SDKs

```python
# Python example
from config_schema import HelixConfig

# Load configuration
config = HelixConfig.from_file('config.hlx')

# Access data
db_host = config.database.host
db_port = config['database']['port']

# Modify configuration
config.database.port = 3306
config.set('database.pool_size', 10)

# Process and compile
config.process()
compiled_data = config.compile()
```

```javascript
// JavaScript example
const HelixConfig = require('./config_schema');

const config = HelixConfig.fromFile('config.hlx');

// Access patterns
const host = config.database.host;
const port = config['database']['port'];

// Method access
config.set('database.pool_size', 10);

// Processing
config.process();
const compiled = config.compile();
```

```rust
// Rust example
use config_schema::HelixConfig;

let config = HelixConfig::from_file("config.hlx").unwrap();

// Access data
let host = config.get("database.host");
config.set("database.port".to_string(), serde_json::json!(3306));

// Process
config.process().unwrap();
let compiled = config.compile().unwrap();
```

## Advanced Usage

### Custom Output Paths

```bash
# Generate multiple SDKs
hlx schema config.hlx -l python -o api/config_sdk.py
hlx schema config.hlx -l java-script -o web/config.js
hlx schema config.hlx -l c-sharp -o dotnet/ConfigSdk.cs
```

### Batch Generation

```bash
# Generate SDKs for all services
for service in api web worker; do
    hlx schema ${service}/config.hlx -l python -o ${service}/config_sdk.py
done
```

### Integration with Build Systems

```makefile
# Makefile example
config_sdk.py: config.hlx
    hlx schema $< -l python -o $@

config_sdk.js: config.hlx
    hlx schema $< -l java-script -o $@
```

```json
// package.json example
{
  "scripts": {
    "generate-sdks": "hlx schema config.hlx -l python -o src/config_sdk.py && hlx schema config.hlx -l java-script -o public/config.js"
  }
}
```

## Best Practices

1. **Version Control**: Include generated SDKs in version control for stability
2. **Regeneration**: Regenerate SDKs when Helix configuration schema changes
3. **Language Selection**: Choose target language based on your application's ecosystem
4. **Error Handling**: Always handle potential parsing/compilation errors
5. **Testing**: Test generated SDKs with your actual configuration data

## Troubleshooting

### Common Issues

**"Parse errors"**
- Check Helix file syntax
- Ensure proper block delimiters (`{}`, `<>`, `[]`, `:`)
- Verify indentation and structure

**"Validation errors"**
- Run `hlx validate <file>` first to check configuration validity
- Ensure all required fields are present
- Check for semantic errors in the configuration

**"Language not supported"**
- Use exact enum values: `rust`, `python`, `java-script`, `c-sharp`, `java`, `go`, `ruby`, `php`
- Check case sensitivity

**"File access denied"**
- Ensure read permissions on input file
- Ensure write permissions on output directory
- Check file paths are correct

### Getting Help

```bash
# Show command help
hlx schema --help

# Validate Helix file first
hlx validate config.hlx

# Check CLI version
hlx --version
```

## Implementation Details

The schema command:
1. Parses the Helix configuration file using the enhanced Helix parser
2. Validates the configuration against semantic rules
3. Analyzes the configuration structure
4. Generates language-specific code with appropriate idioms
5. Writes the generated SDK to the specified output location

The generated code is production-ready and includes:
- Proper error handling
- Type safety (where applicable)
- Documentation comments
- Consistent API across languages
- Memory-safe implementations

## See Also

- `hlx validate` - Validate Helix configuration files
- `hlx compile` - Compile Helix files to binary format
- `hlx decompile` - Convert binary files back to Helix source
- Helix Configuration Language Reference
