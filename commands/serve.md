e err# Helix Serve Command

The `helix serve` command starts a HTTP server for serving HELIX configuration files (.hlx and .hlxb) with automatic conversion capabilities.

## Usage

```bash
helix serve [OPTIONS]
```

## Options

- `-p, --port <PORT>`: Port to listen on (default: 4592)
- `--domain <DOMAIN>`: Domain to bind to (default: localhost)
- `-d, --directory <DIRECTORY>`: Directory to serve (defaults to current directory)
- `--no-convert`: Disable auto-conversion from .hlx to .hlxb
- `--cache-timeout <SECONDS>`: Cache timeout in seconds (default: 300)
- `--max-file-size <BYTES>`: Maximum file size in bytes (default: 10MB)
- `-v, --verbose`: Enable verbose output
- `-h, --help`: Print help information

## Examples

### Basic Usage

```bash
# Start server on default port 4592
helix serve

# Start server on custom port
helix serve --port 8080

# Start server with custom domain and directory
helix serve --domain 0.0.0.0 --directory ./configs

# Start server with verbose output
helix serve --verbose
```

### Advanced Configuration

```bash
# Disable auto-conversion and set custom cache timeout
helix serve --no-convert --cache-timeout 600 --max-file-size 52428800

# Serve from specific directory
helix serve --directory /path/to/configs --port 3000
```

## Endpoints

The server provides the following endpoints:

### GET /

Returns server information and available files in JSON format.

**Example Response:**
```json
{
  "server": {
    "name": "HELIX Configuration Server",
    "domain": "localhost",
    "port": 4592,
    "root_directory": ".",
    "auto_convert": true,
    "cache_timeout": 300,
    "max_file_size": 10485760,
    "cors_enabled": true
  },
  "endpoints": {
    "root": "GET / - Server information (JSON)",
    "health": "GET /health - Health check (JSON)",
    "config": "GET /config - Server configuration (JSON)",
    "files": "GET /filename.hlx - Serve HLX file",
    "binary_files": "GET /filename.hlxb - Serve HLXB file",
    "domain_files": "GET /domain.hlxb - Domain-based configuration"
  },
  "available_files": [
    {
      "name": "config.hlx",
      "name_without_extension": "config",
      "extension": "hlx",
      "size": 1024,
      "url": "/config.hlx"
    }
  ]
}
```

### GET /health

Returns server health status.

**Example Response:**
```json
{
  "status": "healthy",
  "service": "helix-config-server",
  "domain": "localhost",
  "port": 4592,
  "timestamp": "2025-01-07T04:15:30.123456+00:00",
  "uptime": "unknown"
}
```

### GET /config

Returns server configuration.

**Example Response:**
```json
{
  "port": 4592,
  "domain": "localhost",
  "root_directory": ".",
  "auto_convert": true,
  "cache_timeout": 300,
  "max_file_size": 10485760,
  "allowed_extensions": ["hlx", "hlxb"],
  "cors_enabled": true,
  "verbose": false
}
```

### GET /filename.hlx

Serves a HELIX source file (.hlx). If auto-conversion is enabled, the file will be converted to HLXB format before serving.

**Example:**
```bash
curl http://localhost:4592/myconfig.hlx
```

### GET /filename.hlxb

Serves a compiled HELIX binary file (.hlxb).

**Example:**
```bash
curl http://localhost:4592/myconfig.hlxb
```

### GET /domain.hlxb

Serves domain-specific configuration files using the pattern `domain.hlxb`.

**Example:**
```bash
curl http://localhost:4592/myapp.hlxb
```

## Features

### Auto-Conversion

When enabled (default), the server automatically converts .hlx files to .hlxb format when requested. This allows clients to access the optimized binary format without manual compilation.

### Caching

The server implements file caching with configurable timeout (default: 5 minutes) to improve performance for frequently accessed files.

### CORS Support

Cross-Origin Resource Sharing is enabled by default, allowing web applications to access the server from different domains.

### Content-Type Headers

The server sets appropriate Content-Type headers:
- `.hlx` files: `application/x-helix`
- `.hlxb` files: `application/x-helix-binary`

### Error Handling

The server provides proper HTTP error responses with descriptive messages for various error conditions.

## Use Cases

### Development Server

```bash
# Start development server for testing configurations
helix serve --port 3000 --verbose
```

### Production Configuration Server

```bash
# Serve configurations for distributed applications
helix serve --domain 0.0.0.0 --port 80 --directory /etc/helix/configs
```

### API Integration

```bash
# Fetch configuration programmatically
curl -s http://localhost:4592/myapp.hlxb | your-app-parser
```

### Domain-Based Configuration

```bash
# Access domain-specific configs
curl http://config-server.com/myapp.hlxb
```

## Security Considerations

- The server binds to localhost by default for security
- File serving includes security checks to prevent directory traversal
- Maximum file size limits prevent resource exhaustion
- CORS is enabled but can be customized for production use

## Troubleshooting

### Server Won't Start

- Check if the specified port is already in use
- Verify the directory path exists and is readable
- Ensure proper permissions for file access

### Files Not Found

- Verify the file exists in the served directory
- Check file permissions
- Ensure the file extension is .hlx or .hlxb

### Auto-Conversion Issues

- Disable auto-conversion with `--no-convert` flag
- Check server logs for conversion errors
- Verify source .hlx files are valid

### Connection Refused

- Ensure the server is running
- Check the correct host and port
- Verify firewall settings

## Integration Examples

### Rust Application

```rust
use reqwest::blocking;

fn load_config() -> Result<String, Box<dyn std::error::Error>> {
    let response = reqwest::blocking::get("http://localhost:4592/myapp.hlxb")?;
    Ok(response.text()?)
}
```

### Shell Script

```bash
#!/bin/bash
CONFIG=$(curl -s http://localhost:4592/app.hlxb)
echo "Loaded config: $CONFIG"
```

### JavaScript/Node.js

```javascript
const axios = require('axios');

async function loadConfig() {
    const response = await axios.get('http://localhost:4592/config.hlxb');
    return response.data;
}
```

## Performance Tuning

- Adjust cache timeout based on configuration update frequency
- Set appropriate max file size limits for your use case
- Use domain-specific serving for multi-tenant applications
- Enable verbose logging during development for debugging

## Related Commands

- `helix compile`: Compile .hlx files to .hlxb format
- `helix validate`: Validate HELIX configuration files
- `helix init`: Initialize a new HELIX project
