#!/bin/bash

# Helix Cross-Platform Build Script
# Automatically installs dependencies and builds for all platforms
# Supports: Linux, Windows, macOS (Intel/ARM)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OSXCROSS_DIR="osxcross"
PARALLEL_JOBS="${PARALLEL_JOBS:-16}"
SKIP_DEPS="${SKIP_DEPS:-false}"
BUILD_TARGETS="${BUILD_TARGETS:-all}"

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[⚠]${NC} $1"
}

print_error() {
    echo -e "${RED}[✗]${NC} $1"
}

print_header() {
    echo -e "\n${CYAN}=== $1 ===${NC}\n"
}

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)     OS="Linux";;
        Darwin*)    OS="Mac";;
        CYGWIN*|MINGW*|MSYS*) OS="Windows";;
        *)          OS="Unknown";;
    esac
    echo "$OS"
}

# Check if running with sudo (for package installation)
check_sudo() {
    if [[ $EUID -ne 0 ]] && [[ "$SKIP_DEPS" != "true" ]]; then
        print_warning "This script may need sudo for installing dependencies."
        print_status "You can run with SKIP_DEPS=true to skip dependency installation."
        read -p "Continue without sudo? (dependencies won't be auto-installed) [y/N]: " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            print_error "Please run with sudo or set SKIP_DEPS=true"
            exit 1
        fi
        SKIP_DEPS="true"
    fi
}

# Install system dependencies
install_dependencies() {
    if [[ "$SKIP_DEPS" == "true" ]]; then
        print_warning "Skipping dependency installation (SKIP_DEPS=true)"
        return
    fi

    print_header "Installing System Dependencies"
    
    local OS=$(detect_os)
    
    case "$OS" in
        Linux)
            if command -v apt-get &> /dev/null; then
                print_status "Installing dependencies via apt..."
                sudo apt-get update
                sudo apt-get install -y \
                    build-essential \
                    curl \
                    git \
                    pkg-config \
                    libssl-dev \
                    musl-tools \
                    mingw-w64 \
                    gcc-mingw-w64-x86-64 \
                    gcc-mingw-w64-i686 \
                    clang \
                    cmake \
                    libxml2-dev \
                    libz-dev \
                    libtinfo-dev
            elif command -v yum &> /dev/null; then
                print_status "Installing dependencies via yum..."
                sudo yum groupinstall -y "Development Tools"
                sudo yum install -y \
                    curl \
                    git \
                    openssl-devel \
                    mingw64-gcc \
                    mingw32-gcc \
                    clang \
                    cmake
            elif command -v pacman &> /dev/null; then
                print_status "Installing dependencies via pacman..."
                sudo pacman -Syu --noconfirm \
                    base-devel \
                    curl \
                    git \
                    openssl \
                    musl \
                    mingw-w64-gcc \
                    clang \
                    cmake
            else
                print_warning "Unknown package manager. Please install dependencies manually."
            fi
            ;;
        Mac)
            if command -v brew &> /dev/null; then
                print_status "Installing dependencies via Homebrew..."
                brew install cmake libxml2 openssl
            else
                print_warning "Homebrew not found. Please install dependencies manually."
            fi
            ;;
        *)
            print_warning "Unsupported OS for automatic dependency installation"
            ;;
    esac
}

# Check and install Rust
install_rust() {
    print_header "Checking Rust Installation"
    
    if ! command -v rustup &> /dev/null; then
        print_status "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    else
        print_success "Rust is already installed"
    fi
    
    # Update Rust
    print_status "Updating Rust toolchain..."
    rustup update stable
}

# Setup osxcross for macOS cross-compilation
setup_osxcross() {
    print_header "Setting up OSXCross for macOS Cross-Compilation"
    
    if [[ -d "$OSXCROSS_DIR" ]] && [[ -f "$OSXCROSS_DIR/target/bin/x86_64-apple-darwin-clang" ]]; then
        print_success "OSXCross already installed at $OSXCROSS_DIR"
        export PATH="$OSXCROSS_DIR/target/bin:$PATH"
        return
    fi
    
    print_status "OSXCross not found. Setting up..."
    
    # Clone osxcross
    if [[ ! -d "$OSXCROSS_DIR" ]]; then
        git clone https://github.com/tpoechtrager/osxcross "$OSXCROSS_DIR"
    fi
    
    cd "$OSXCROSS_DIR"
    
    # Check for SDK
    if ! ls tarballs/MacOSX*.tar.* &> /dev/null; then
        print_warning "macOS SDK not found!"
        print_status "Please download a macOS SDK and place it in $OSXCROSS_DIR/tarballs/"
        print_status "You can get it from: https://github.com/phracker/MacOSX-SDKs/releases"
        print_status "Recommended: MacOSX11.3.sdk.tar.xz or later"
        read -p "Press Enter when ready, or 's' to skip macOS builds: " -n 1 -r
        echo
        if [[ $REPLY == "s" ]]; then
            print_warning "Skipping macOS builds"
            return 1
        fi
    fi
    
    # Build osxcross
    print_status "Building OSXCross (this may take a while)..."
    UNATTENDED=1 ./build.sh
    
    # Add to PATH
    export PATH="$OSXCROSS_DIR/target/bin:$PATH"
    
    cd "$SCRIPT_DIR"
    print_success "OSXCross setup complete"
}

# Install Rust targets
install_rust_targets() {
    print_header "Installing Rust Cross-Compilation Targets"
    
    local targets=(
        "x86_64-unknown-linux-gnu"
        "x86_64-unknown-linux-musl"
        "aarch64-unknown-linux-gnu"
        "aarch64-unknown-linux-musl"
        "x86_64-pc-windows-gnu"
        "i686-pc-windows-gnu"
        "x86_64-apple-darwin"
        "aarch64-apple-darwin"
    )
    
    for target in "${targets[@]}"; do
        if rustup target list --installed | grep -q "^$target\$"; then
            print_success "Target $target already installed"
        else
            print_status "Installing target: $target"
            if rustup target add "$target" 2>/dev/null; then
                print_success "Installed $target"
            else
                print_warning "Could not install $target (may not be available on this host)"
            fi
        fi
    done
}

# Setup cargo config for cross-compilation
setup_cargo_config() {
    print_header "Setting up Cargo Configuration"
    
    mkdir -p .cargo
    
    cat > .cargo/config.toml << EOF
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
ar = "x86_64-w64-mingw32-ar"

[target.i686-pc-windows-gnu]
linker = "i686-w64-mingw32-gcc"
ar = "i686-w64-mingw32-ar"

[target.x86_64-unknown-linux-musl]
linker = "musl-gcc"

[target.aarch64-unknown-linux-musl]
linker = "aarch64-linux-musl-gcc"
EOF

    # Add macOS config if osxcross is available
    if [[ -f "$OSXCROSS_DIR/target/bin/x86_64-apple-darwin-clang" ]]; then
        cat >> .cargo/config.toml << EOF

[target.x86_64-apple-darwin]
linker = "$OSXCROSS_DIR/target/bin/x86_64-apple-darwin-clang"
ar = "$OSXCROSS_DIR/target/bin/x86_64-apple-darwin-ar"

[target.aarch64-apple-darwin]
linker = "$OSXCROSS_DIR/target/bin/aarch64-apple-darwin-clang"
ar = "$OSXCROSS_DIR/target/bin/aarch64-apple-darwin-ar"
EOF
        # Set environment variables for macOS cross-compilation
        export CC_x86_64_apple_darwin="$OSXCROSS_DIR/target/bin/o64-clang"
        export CXX_x86_64_apple_darwin="$OSXCROSS_DIR/target/bin/o64-clang++"
        export AR_x86_64_apple_darwin="$OSXCROSS_DIR/target/bin/x86_64-apple-darwin-ar"
        export CC_aarch64_apple_darwin="$OSXCROSS_DIR/target/bin/aarch64-apple-darwin-clang"
        export CXX_aarch64_apple_darwin="$OSXCROSS_DIR/target/bin/aarch64-apple-darwin-clang++"
        export AR_aarch64_apple_darwin="$OSXCROSS_DIR/target/bin/aarch64-apple-darwin-ar"
    fi
    
    print_success "Cargo configuration created"
}

# Build for a specific target
build_target() {
    local target=$1
    local binary_name="helix"
    
    # Determine binary name
    case "$target" in
        *windows*)
            binary_name="helix.exe"
            ;;
    esac
    
    print_status "Building for $target..."
    
    # Set up environment for specific targets
    case "$target" in
        *apple-darwin*)
            if [[ ! -f "$OSXCROSS_DIR/target/bin/x86_64-apple-darwin-clang" ]]; then
                print_warning "OSXCross not available, skipping $target"
                return 1
            fi
            ;;
        *musl*)
            if ! command -v musl-gcc &> /dev/null; then
                print_warning "musl-gcc not found, skipping $target"
                return 1
            fi
            ;;
        *windows*)
            if ! command -v x86_64-w64-mingw32-gcc &> /dev/null && [[ "$target" == "x86_64-pc-windows-gnu" ]]; then
                print_warning "mingw-w64 not found, skipping $target"
                return 1
            fi
            ;;
    esac
    
    # Build
    if cargo build --release --features full --target "$target" --jobs "$PARALLEL_JOBS" 2>&1 | tee build_"$target".log; then
        # Copy binary to releases
        platform_name=$(echo "$target" | sed 's/-/_/g')
        if [[ -f "target/$target/release/$binary_name" ]]; then
            cp -f "target/$target/release/$binary_name" "web/src/releases/helix_$platform_name"
            
            # Strip binary to reduce size (except for Windows)
            if [[ "$target" != *windows* ]] && command -v strip &> /dev/null; then
                strip "web/src/releases/helix_$platform_name" 2>/dev/null || true
            fi
            
            print_success "Successfully built $target"
            return 0
        else
            print_error "Binary not found for $target"
            return 1
        fi
    else
        print_error "Build failed for $target (see build_$target.log for details)"
        return 1
    fi
}

# Main build function
build_all() {
    print_header "Building Helix for All Platforms"
    
    # Create releases directory
    rm -rf web/src/releases
    mkdir -p web/src/releases
    
    # Define build targets
    local targets
    
    if [[ "$BUILD_TARGETS" == "all" ]]; then
        targets=(
            "x86_64-unknown-linux-gnu"
            "x86_64-unknown-linux-musl"
            "x86_64-pc-windows-gnu"
            "i686-pc-windows-gnu"
            "x86_64-apple-darwin"
            "aarch64-apple-darwin"
            # "aarch64-unknown-linux-gnu"  # Requires additional setup
            # "aarch64-unknown-linux-musl" # Requires additional setup
        )
    else
        IFS=',' read -ra targets <<< "$BUILD_TARGETS"
    fi
    
    local successful=0
    local failed=0
    local skipped=0
    
    for target in "${targets[@]}"; do
        if build_target "$target"; then
            ((successful++))
        else
            ((failed++))
        fi
    done
    
    print_header "Build Summary"
    print_success "Successful builds: $successful"
    [[ $failed -gt 0 ]] && print_error "Failed builds: $failed"
    [[ $skipped -gt 0 ]] && print_warning "Skipped builds: $skipped"
}

# Create distribution package
create_distribution() {
    print_header "Creating Distribution Package"
    
    if [[ ! -d "web/src/releases" ]] || [[ -z "$(ls -A web/src/releases)" ]]; then
        print_error "No binaries found in web/src/releases"
        return 1
    fi
    
    # Create tar.gz with all binaries and install script
    cd web/src
    
    # Create a simple install script if it doesn't exist
    if [[ ! -f "../../install.sh" ]]; then
        cat > ../../install.sh << 'EOF'
#!/bin/bash
# Helix Installation Script

INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
ARCH=$(uname -m)
OS=$(uname -s | tr '[:upper:]' '[:lower:]')

# Determine the correct binary
case "$OS" in
    linux)
        case "$ARCH" in
            x86_64) BINARY="helix_x86_64_unknown_linux_gnu" ;;
            aarch64) BINARY="helix_aarch64_unknown_linux_gnu" ;;
            *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
        esac
        ;;
    darwin)
        case "$ARCH" in
            x86_64) BINARY="helix_x86_64_apple_darwin" ;;
            arm64) BINARY="helix_aarch64_apple_darwin" ;;
            *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
        esac
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

if [[ -f "releases/$BINARY" ]]; then
    echo "Installing $BINARY to $INSTALL_DIR/helix"
    sudo cp "releases/$BINARY" "$INSTALL_DIR/helix"
    sudo chmod +x "$INSTALL_DIR/helix"
    echo "Helix installed successfully!"
else
    echo "Binary not found: releases/$BINARY"
    exit 1
fi
EOF
        chmod +x install.sh
    else
        cp ../../install.sh .
        chmod +x ../../install.sh
    fi
    
    # Create tarball
    tar -czf ../../get/latest.tar.gz releases . -C ../.. install.sh
    cd "$SCRIPT_DIR"
    
    print_success "Distribution created: get/latest.tar.gz"
    print_status "File size: $(du -h get/latest.tar.gz | cut -f1)"
    
    # List included binaries
    print_status "Included binaries:"
    tar -tzf get/latest.tar.gz | grep releases/ | sed 's|releases/||' | while read -r file; do
        echo "  - $file"
    done
}

# Show usage
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Options:
    -h, --help              Show this help message
    -s, --skip-deps         Skip dependency installation
    -t, --targets TARGETS   Comma-separated list of targets to build
                           (default: all)
    -j, --jobs N           Number of parallel jobs (default: 16)
    -o, --osxcross DIR     OSXCross installation directory
                           (default: ~/osxcross)
    
Environment Variables:
    SKIP_DEPS=true         Skip dependency installation
    BUILD_TARGETS=...      Comma-separated list of targets
    PARALLEL_JOBS=N        Number of parallel jobs
    OSXCROSS_DIR=PATH      OSXCross installation directory

Available Targets:
    x86_64-unknown-linux-gnu
    x86_64-unknown-linux-musl
    x86_64-pc-windows-gnu
    i686-pc-windows-gnu
    x86_64-apple-darwin
    aarch64-apple-darwin
    aarch64-unknown-linux-gnu
    aarch64-unknown-linux-musl

Examples:
    # Build all targets with auto-installation
    sudo $0
    
    # Build only Linux and Windows targets
    $0 --targets x86_64-unknown-linux-gnu,x86_64-pc-windows-gnu
    
    # Skip dependency installation
    $0 --skip-deps
    
    # Use existing osxcross installation
    $0 --osxcross /opt/osxcross
EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_usage
                exit 0
                ;;
            -s|--skip-deps)
                SKIP_DEPS="true"
                shift
                ;;
            -t|--targets)
                BUILD_TARGETS="$2"
                shift 2
                ;;
            -j|--jobs)
                PARALLEL_JOBS="$2"
                shift 2
                ;;
            -o|--osxcross)
                OSXCROSS_DIR="$2"
                shift 2
                ;;
            *)
                print_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done
}

# Main execution
main() {
    print_header "Helix Cross-Platform Build System"
    
    # Parse arguments
    parse_args "$@"
    
    # Check sudo
    check_sudo
    
    # Install dependencies
    install_dependencies
    
    # Install Rust
    install_rust
    
    # Install Rust targets
    install_rust_targets
    
    # Setup osxcross
    #if [[ "$BUILD_TARGETS" == "all" ]] || [[ "$BUILD_TARGETS" == *"apple"* ]]; then
        #setup_osxcross || print_warning "Continuing without macOS support"
    #fi
    
    # Setup cargo config
    setup_cargo_config
    
    # Build all targets
    build_all
    
    # Create distribution
    create_distribution
    
    print_header "Build Complete! 🎉"
    print_status "Distribution available at: get/latest.tar.gz"
    print_status "Build logs available in: build_*.log"
}

# Run main function
main "$@"