#!/bin/bash

# OSXCross Quick Setup Script
# Automates the installation of osxcross for macOS cross-compilation

set -e

OSXCROSS_DIR="${1:-$HOME/osxcross}"
SDK_VERSION="${SDK_VERSION:-11.3}"

echo "🍎 OSXCross Quick Setup"
echo "Installation directory: $OSXCROSS_DIR"

# Install dependencies
echo "📦 Installing dependencies..."
if command -v apt-get &> /dev/null; then
    sudo apt-get update
    sudo apt-get install -y \
        clang \
        cmake \
        git \
        patch \
        python3 \
        libssl-dev \
        lzma-dev \
        libxml2-dev \
        libbz2-dev \
        xz-utils \
        cpio \
        zlib1g-dev
elif command -v yum &> /dev/null; then
    sudo yum install -y \
        clang \
        cmake \
        git \
        patch \
        python3 \
        openssl-devel \
        xz-devel \
        libxml2-devel \
        bzip2-devel
fi

# Clone osxcross
if [[ ! -d "$OSXCROSS_DIR" ]]; then
    echo "📥 Cloning osxcross..."
    git clone https://github.com/tpoechtrager/osxcross "$OSXCROSS_DIR"
fi

cd "$OSXCROSS_DIR"

# Download SDK if not present
if ! ls tarballs/MacOSX*.tar.* &> /dev/null 2>&1; then
    echo "📥 Downloading macOS SDK ${SDK_VERSION}..."
    mkdir -p tarballs
    
    # Try to download from common sources
    SDK_URL="https://github.com/phracker/MacOSX-SDKs/releases/download/${SDK_VERSION}/MacOSX${SDK_VERSION}.sdk.tar.xz"
    
    if wget -q --spider "$SDK_URL" 2>/dev/null; then
        wget -O "tarballs/MacOSX${SDK_VERSION}.sdk.tar.xz" "$SDK_URL"
    else
        echo "⚠️  Could not download SDK automatically."
        echo "Please download MacOSX${SDK_VERSION}.sdk.tar.xz manually from:"
        echo "  https://github.com/phracker/MacOSX-SDKs/releases"
        echo "And place it in: $OSXCROSS_DIR/tarballs/"
        exit 1
    fi
fi

# Build osxcross
echo "🔨 Building osxcross (this will take a few minutes)..."
UNATTENDED=1 ./build.sh

# Create environment setup script
cat > "$OSXCROSS_DIR/env.sh" << 'EOF'
#!/bin/bash
export PATH="$OSXCROSS_DIR/target/bin:$PATH"
export CC_x86_64_apple_darwin="$OSXCROSS_DIR/target/bin/o64-clang"
export CXX_x86_64_apple_darwin="$OSXCROSS_DIR/target/bin/o64-clang++"
export AR_x86_64_apple_darwin="$OSXCROSS_DIR/target/bin/x86_64-apple-darwin-ar"
export CC_aarch64_apple_darwin="$OSXCROSS_DIR/target/bin/aarch64-apple-darwin-clang"
export CXX_aarch64_apple_darwin="$OSXCROSS_DIR/target/bin/aarch64-apple-darwin-clang++"
export AR_aarch64_apple_darwin="$OSXCROSS_DIR/target/bin/aarch64-apple-darwin-ar"
echo "OSXCross environment configured!"
EOF

sed -i "s|\$OSXCROSS_DIR|$OSXCROSS_DIR|g" "$OSXCROSS_DIR/env.sh"
chmod +x "$OSXCROSS_DIR/env.sh"

echo "✅ OSXCross installation complete!"
echo ""
echo "To use osxcross in your shell:"
echo "  source $OSXCROSS_DIR/env.sh"
echo ""
echo "To use with the build script:"
echo "  OSXCROSS_DIR=$OSXCROSS_DIR ./build.sh"
echo ""
echo "Add this to your ~/.bashrc or ~/.zshrc for permanent setup:"
echo "  export OSXCROSS_DIR=$OSXCROSS_DIR"
echo "  export PATH=\"\$OSXCROSS_DIR/target/bin:\$PATH\""