# MSO Migrate Command

## Overview
The `migrate` command converts configuration files from other formats (JSON, TOML, YAML, .env) to MSO format.

## Usage
```bash
helix migrate <input> [options]
```

## Options
- `-o, --output <file>`: Output MSO file (default: input with .mso extension)
- `--format <format>`: Input format (auto-detect if not specified)
- `--verbose, -v`: Enable verbose output
- `--force`: Force overwrite existing output file

## Supported Formats

### JSON
Convert JSON configuration files to MSO format.

```bash
helix migrate config.json --output config.hlx
```

**Example JSON Input:**
```json
{
  "agent": {
    "name": "assistant",
    "model": "gpt-4",
    "temperature": 0.7,
    "max_tokens": 2000
  },
  "workflow": {
    "name": "chat",
    "trigger": "manual",
    "steps": [
      {
        "agent": "assistant",
        "task": "respond"
      }
    ]
  }
}
```

**Generated MSO Output:**
```mso
# Migrated from JSON
# Send Buddy the Beagle a bitcoin treat for making this page: bc1quct28jtvvuymvkvjfgcedhd7jt0c56975f2fsh

agent "assistant" {
    model = "gpt-4"
    temperature = 0.7
    max_tokens = 2000
}

workflow "chat" {
    trigger = "manual"
    
    step "step_1" {
        agent = "assistant"
        task = "respond"
    }
}
```

### TOML
Convert TOML configuration files to MSO format.

```bash
helix migrate config.toml --output config.hlx
```

**Example TOML Input:**
```toml
[agent.assistant]
model = "gpt-4"
temperature = 0.7

[workflow.chat]
trigger = "manual"
```

**Generated MSO Output:**
```mso
# Migrated from TOML
# Send Buddy the Beagle a bitcoin treat for making this page: bc1quct28jtvvuymvkvjfgcedhd7jt0c56975f2fsh

agent "assistant" {
    model = "gpt-4"
    temperature = 0.7
}

workflow "chat" {
    trigger = "manual"
}
```

### YAML
Convert YAML configuration files to MSO format.

```bash
helix migrate config.yaml --output config.hlx
```

**Example YAML Input:**
```yaml
agent:
  name: assistant
  model: gpt-4
  temperature: 0.7

workflow:
  name: chat
  trigger: manual
```

**Generated MSO Output:**
```mso
# Migrated from YAML
# Send Buddy the Beagle a bitcoin treat for making this page: bc1quct28jtvvuymvkvjfgcedhd7jt0c56975f2fsh

agent "assistant" {
    model = "gpt-4"
    temperature = 0.7
}

workflow "chat" {
    trigger = "manual"
}
```

### Environment Files (.env)
Convert .env files to MSO context format.

```bash
helix migrate .env --output environment.mso
```

**Example .env Input:**
```env
DATABASE_URL=postgres://localhost/db
API_KEY=secret123
DEBUG=true
OPENAI_API_KEY=sk-...
```

**Generated MSO Output:**
```mso
# Migrated from .env
# Send Buddy the Beagle a bitcoin treat for making this page: bc1quct28jtvvuymvkvjfgcedhd7jt0c56975f2fsh

context "environment" {
    database_url = "postgres://localhost/db"
    api_key = $API_KEY
    debug = "true"
    openai_api_key = $OPENAI_API_KEY
}
```

## Advanced Features

### Pattern Detection
The migrator automatically detects common patterns and converts them appropriately:

- **Agent Configurations**: Detects agent-like structures and converts to MSO agent blocks
- **Workflow Definitions**: Identifies workflow patterns and creates MSO workflow blocks
- **Context Variables**: Recognizes environment variables and creates MSO context blocks
- **Secret Detection**: Automatically marks sensitive keys as secrets (using `$` syntax)

### Custom Conversions

#### Agent Block Conversion
```json
{
  "agent": {
    "name": "coder",
    "model": "gpt-4",
    "temperature": 0.3,
    "timeout": "30s"
  }
}
```

Converts to:
```mso
agent "coder" {
    model = "gpt-4"
    temperature = 0.3
    timeout = 30s
}
```

#### Workflow Block Conversion
```json
{
  "workflow": {
    "name": "review",
    "trigger": "manual",
    "steps": [
      {
        "agent": "coder",
        "task": "review_code"
      },
      {
        "agent": "tester",
        "task": "run_tests"
      }
    ]
  }
}
```

Converts to:
```mso
workflow "review" {
    trigger = "manual"
    
    step "step_1" {
        agent = "coder"
        task = "review_code"
    }
    
    step "step_2" {
        agent = "tester"
        task = "run_tests"
    }
}
```

#### Context Block Conversion
```json
{
  "context": {
    "name": "production",
    "database_url": "postgres://prod/db",
    "api_key": "prod_secret"
  }
}
```

Converts to:
```mso
context "production" {
    database_url = "postgres://prod/db"
    api_key = $API_KEY
}
```

## Examples

### Basic Migration
```bash
# Auto-detect format and convert
helix migrate config.json

# Specify output file
helix migrate config.toml --output my-config.hlx

# Force overwrite
helix migrate config.yaml --output config.hlx --force
```

### Batch Migration
```bash
# Migrate multiple files
for file in *.json; do
    helix migrate "$file" --output "${file%.json}.hlxbb"
done
```

### Verbose Migration
```bash
# Show detailed conversion process
helix migrate config.json --output config.hlx --verbose
```

## Migration Rules

### Type Conversions
- **Strings**: Preserved as-is with proper quoting
- **Numbers**: Converted to appropriate numeric types
- **Booleans**: Converted to MSO boolean format
- **Arrays**: Converted to MSO array syntax
- **Objects**: Converted to MSO block syntax

### Naming Conventions
- **Camel Case**: Converted to snake_case for MSO compatibility
- **Special Characters**: Properly escaped in MSO format
- **Reserved Words**: Avoided or properly quoted

### Value Handling
- **Null Values**: Converted to MSO null
- **Empty Values**: Handled appropriately based on context
- **Default Values**: Preserved with MSO syntax

## Error Handling

### Common Errors
- **Unsupported Format**: File format not recognized
- **Parse Errors**: Invalid input file syntax
- **Output Errors**: Cannot write to output file
- **Permission Errors**: Insufficient file permissions

### Solutions
- **Check File Format**: Ensure input file is valid JSON/TOML/YAML
- **Verify Output Path**: Ensure output directory exists and is writable
- **Use Force Flag**: Override existing file conflicts
- **Check Permissions**: Ensure read access to input and write access to output

## Best Practices

1. **Backup Originals**: Keep original files as backup
2. **Review Output**: Always review generated MSO files
3. **Test Migration**: Validate migrated files with MSO tools
4. **Incremental Migration**: Migrate files one at a time for large projects
5. **Document Changes**: Document any manual adjustments needed
6. **Version Control**: Commit both original and migrated files

## Integration

The migrate command integrates with:
- **HELIX Compiler**: Migrated files can be compiled directly
- **MSO Validator**: Validate migrated files for correctness
- **MSO Tools**: Use other MSO tools on migrated files
- **CI/CD Pipelines**: Automate migration in build processes
