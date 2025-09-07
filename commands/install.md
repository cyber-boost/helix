# MSO Install Command

## Overview
The `install` command globally installs the MSO compiler to your system, making it available from anywhere in your terminal. It creates the baton directory structure and sets up global symlinks for easy access.

## Usage
```bash
mso install [options]
```

## Options

### `--local-only`
- **Description**: Skip global symlink creation, only install to ~/.baton/bin/
- **Default**: Disabled (creates global symlinks)
- **Example**: `mso install --local-only`

### `-f, --force`
- **Description**: Force overwrite existing installation
- **Default**: Disabled (prevents accidental overwrites)
- **Example**: `mso install --force`

### `-v, --verbose`
- **Description**: Show detailed installation steps and information
- **Default**: Disabled
- **Example**: `mso install --verbose`

## Examples

### Basic Installation
```bash
mso install
# Installs MSO globally with automatic symlink creation
```

### Local Installation Only
```bash
mso install --local-only
# Installs only to ~/.baton/bin/ without global symlinks
```

### Force Reinstall
```bash
mso install --force
# Overwrites existing installation
```

### Verbose Installation
```bash
mso install --verbose
# Shows detailed installation process
```

## Installation Process

### Directory Creation
- **Baton Directory**: Creates `$HOME/.baton/` directory
- **Bin Directory**: Creates `$HOME/.baton/bin/` directory
- **Permissions**: Sets appropriate directory permissions

### Binary Installation
- **Source Detection**: Automatically detects current executable path
- **Binary Copy**: Copies MSO compiler to `$HOME/.baton/bin/mso`
- **Permissions**: Sets executable permissions (755) on Unix systems
- **Integrity**: Ensures binary is properly installed and executable

### Global Symlink Creation
The install command attempts to create global symlinks in multiple locations:

1. **`/usr/local/bin`** - Standard Unix location
2. **`/usr/bin`** - System-wide location
3. **`/opt/homebrew/bin`** - Homebrew on Apple Silicon Mac
4. **`/home/linuxbrew/.linuxbrew/bin`** - Homebrew on Linux

The command tries each location until it finds one where it can successfully create a symlink.

## Installation Results

### Successful Global Installation
```
✅ MSO compiler installed successfully!
  Location: /Users/username/.baton/bin/mso
  ✅ Created global symlink: /opt/homebrew/bin/mso

🎉 Global installation complete!
  You can now use 'mso' command from anywhere
  Try: mso --help
```

### Local Installation Only
```
✅ MSO compiler installed successfully!
  Location: /Users/username/.baton/bin/mso

📋 Local installation complete!
  Add /Users/username/.baton/bin to your PATH to use 'mso' command
  Or run: export PATH="/Users/username/.baton/bin:$PATH"
```

### Partial Installation (Symlink Failed)
```
✅ MSO compiler installed successfully!
  Location: /Users/username/.baton/bin/mso

📋 Installation complete, but global symlink creation failed
  This might be due to insufficient permissions
  You can still use MSO by adding /Users/username/.baton/bin to your PATH
  Or run: export PATH="/Users/username/.baton/bin:$PATH"

💡 To create global symlink manually:
  sudo ln -sf /Users/username/.baton/bin/mso /usr/local/bin/mso
```

## Directory Structure
After installation, the following structure is created:
```
$HOME/
└── .baton/
    └── bin/
        └── mso          # MSO compiler binary (executable)
```

## Error Handling
- **Existing Installation**: Prevents overwrite unless `--force` is used
- **Permission Errors**: Gracefully handles symlink creation failures
- **Directory Creation**: Automatically creates necessary directories
- **Binary Copying**: Provides clear error messages for copy failures
- **Path Resolution**: Handles HOME directory resolution errors

## Post-Installation

### Global Access
If global symlinks were created successfully:
```bash
mso --version
# Should work from anywhere
```

### Local Access Only
If using local-only installation or symlinks failed:
```bash
# Add to PATH temporarily
export PATH="$HOME/.baton/bin:$PATH"

# Or add to shell profile permanently
echo 'export PATH="$HOME/.baton/bin:$PATH"' >> ~/.bashrc
echo 'export PATH="$HOME/.baton/bin:$PATH"' >> ~/.zshrc
```

### Manual Symlink Creation
If automatic symlink creation failed:
```bash
# Create symlink manually (requires sudo)
sudo ln -sf $HOME/.baton/bin/mso /usr/local/bin/mso
```

## Use Cases
- **Development.*HELIX compiler for development work
- **System Administration**: Make MSO available system-wide
- **CI/CD**: Install MSO in automated environments
- **Distribution**: Share MSO compiler with team members
- **Portable Installation**: Local installation for restricted environments

## Best Practices
- **Test Installation**: Verify `mso --version` works after installation
- **Update PATH**: Add ~/.baton/bin to PATH for local installations
- **Version Control**: Track MSO compiler versions in projects
- **Backup**: Keep original installer for reinstallation
- **Documentation**: Document installation process for team members

## Troubleshooting

### Permission Denied
- Use `--local-only` for restricted environments
- Add ~/.baton/bin to PATH manually
- Contact system administrator for global installation

### Command Not Found
- Verify PATH includes ~/.baton/bin
- Check shell profile configuration
- Restart terminal after PATH changes

### Symlink Issues
- Use `--local-only` to skip symlink creation
- Create symlinks manually with sudo
- Check available global bin directories

## Uninstallation
To remove MSO installation:
```bash
# Remove binary
rm -f $HOME/.baton/bin/mso

# Remove global symlinks
sudo rm -f /usr/local/bin/mso
sudo rm -f /usr/bin/mso
sudo rm -f /opt/homebrew/bin/mso

# Remove directories (if empty)
rmdir $HOME/.baton/bin
rmdir $HOME/.baton
```
