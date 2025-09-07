# MSO Init Command

## Overview
The `init` command initializes new MSO projects by creating configuration files from embedded templates. This provides a quick start for common AI system patterns and use cases.

## Usage
```bash
mso init [options]
```

## Options

### `-t, --template <TEMPLATE>`
- **Description**: Choose from available MSO templates
- **Default**: `minimal`
- **Available Templates**:
  - `minimal`: Simple MSO configuration with basic agent and workflow
  - `ai-dev`: Complete AI development team with specialized agents
  - `support`: Multi-tier customer support system with escalation
  - `data-pipeline`: High-throughput data processing pipeline with ML
  - `research`: AI-powered research assistant for literature review
- **Example**: `mso init --template ai-dev`

### `-d, --dir <DIRECTORY>`
- **Description**: Output directory for the new MSO file
- **Default**: Current directory
- **Example**: `mso init --dir ./my-project`

### `-n, --name <FILENAME>`
- **Description**: Custom filename for the output MSO file
- **Default**: Template name with `.mso` extension
- **Example**: `mso init --name my-config.hlx`

### `-f, --force`
- **Description**: Overwrite existing files without prompting
- **Default**: Disabled (prevents accidental overwrites)
- **Example**: `mso init --force`

### `-v, --verbose`
- **Description**: Show detailed initialization information
- **Default**: Disabled
- **Example**: `mso init --verbose`

## Examples

### Basic Initialization
```bash
mso init
# Creates minimal.mso in current directory
```

### AI Development Team
```bash
mso init --template ai-dev
# Creates ai_development_team.mso with full AI team configuration
```

### Custom Project Setup
```bash
mso init --template support --name customer-support.mso --dir ./support-systems
# Creates customer-support.mso in ./support-systems directory
```

### Force Overwrite
```bash
mso init --template minimal --force
# Overwrites existing minimal.hlx file
```

### Verbose Initialization
```bash
mso init --template data-pipeline --verbose
# Shows detailed information about the data pipeline template
```

## Available Templates

### Minimal Template
- **File**: `minimal.mso`
- **Size**: ~433 bytes
- **Description**: Simple MSO configuration with basic agent and workflow
- **Use Case**: Learning MSO syntax, simple automation tasks
- **Features**:
  - Basic project declaration
  - Simple agent with GPT-3.5-turbo
  - Basic workflow with manual trigger

### AI Development Team Template
- **File**: `ai_development_team.mso`
- **Size**: ~8,981 bytes
- **Description**: Complete AI development team with specialized agents
- **Use Case**: Full-stack AI development, complex software projects
- **Features**:
  - 5 specialized agents (architect, Rust engineer, frontend, QA, DevOps)
  - Full development workflow with dependencies
  - Code review pipeline
  - Development and QA crews
  - Memory configuration with PostgreSQL
  - Production and development contexts

### Customer Support Template
- **File**: `customer_support.mso`
- **Size**: ~7,500+ bytes
- **Description**: Multi-tier customer support system with escalation
- **Use Case**: Customer service automation, support ticket management
- **Features**:
  - 3-tier support system (Tier 1, 2, 3)
  - Customer success manager
  - Ticket resolution workflow
  - Proactive monitoring workflow
  - Knowledge base update workflow
  - Support team crews
  - Memory with Elasticsearch

### Data Pipeline Template
- **File**: `data_pipeline.mso`
- **Size**: ~6,000+ bytes
- **Description**: High-throughput data processing pipeline with ML
- **Use Case**: Data processing, ML pipelines, real-time analytics
- **Features**:
  - Data ingestion specialist
  - Data transformation expert
  - ML engineer
  - Streaming pipeline workflow
  - Batch analytics workflow
  - Quality monitoring workflow
  - Data team crew
  - Memory with Redis

### Research Assistant Template
- **File**: `research_assistant.mso`
- **Size**: ~5,000+ bytes
- **Description**: AI-powered research assistant for literature review
- **Use Case**: Academic research, paper writing, literature analysis
- **Features**:
  - Literature review specialist
  - Statistical analyst
  - Academic writer
  - Research paper workflow
  - Citation management workflow
  - Research team crew
  - Memory with SQLite

## Output Information

### Basic Output
- ✅ Success message with created file path
- Template used
- Next steps guidance

### Verbose Output
- **Initialization Details**:
  - Template selected
  - Output path
  - Force flag status
- **Template Information**:
  - File size
  - Template description
  - Use case information
- **Next Steps**:
  - Review and customize configuration
  - Set up API keys and environment variables
  - Compile with `helix compile`
  - Run with MSO runtime

## Error Handling
- **Invalid Template**: Clear error with list of available templates
- **File Exists**: Prevents overwrite unless `--force` is used
- **Directory Creation**: Automatically creates directories if needed
- **Permission Errors**: File system access issues

## Use Cases
- **Quick Start**: Rapidly bootstrap new MSO projects
- **Learning**: Study different MSO configuration patterns
- **Prototyping**: Test different AI system architectures
- **Documentation**: Use templates as reference implementations
- **Onboarding**: Help new users understand MSO capabilities

## Best Practices
- **Start Simple**: Begin with minimal template for learning
- **Customize**: Modify templates to fit specific needs
- **Version Control**: Track template customizations
- **Documentation**: Document customizations and decisions
- **Testing**: Validate generated configurations before use

## Template Customization
After initialization, templates can be customized:
- **API Keys**: Add your API keys to contexts
- **Models**: Change AI models to suit your needs
- **Workflows**: Modify workflows for specific use cases
- **Agents**: Customize agent capabilities and backstories
- **Memory**: Configure memory providers and connections
