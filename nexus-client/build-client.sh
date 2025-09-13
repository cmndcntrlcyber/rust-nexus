#!/bin/bash

# Nexus Client Build Script
# Builds the Tauri-based desktop client for all platforms

set -e

echo "🚀 Building Nexus C2 Client..."
echo "=================================="

# Check if we're in the right directory
if [ ! -f "package.json" ]; then
    echo "❌ Error: This script must be run from the nexus-client directory"
    exit 1
fi

# Check dependencies
echo "📦 Checking dependencies..."

# Check Node.js
if ! command -v node &> /dev/null; then
    echo "❌ Node.js is not installed. Please install Node.js 16+"
    exit 1
fi

# Check npm
if ! command -v npm &> /dev/null; then
    echo "❌ npm is not installed. Please install npm"
    exit 1
fi

# Check Rust (look in common locations)
RUST_FOUND=false
if command -v cargo &> /dev/null; then
    RUST_FOUND=true
elif [ -f "$HOME/.cargo/bin/cargo" ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
    RUST_FOUND=true
elif [ -f "/usr/local/cargo/bin/cargo" ]; then
    export PATH="/usr/local/cargo/bin:$PATH"
    RUST_FOUND=true
fi

if [ "$RUST_FOUND" = false ]; then
    echo "❌ Rust is not installed. Please install Rust"
    echo "   Install with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo "   Then run: source ~/.cargo/env"
    exit 1
fi

# Check Tauri CLI
if ! command -v tauri &> /dev/null; then
    echo "📥 Installing Tauri CLI..."
    cargo install tauri-cli
fi

echo "✅ All dependencies found"

# Check and install Linux system dependencies for Tauri
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    echo "🐧 Checking Linux system dependencies for Tauri..."

    # Check if running on Ubuntu/Debian
    if command -v apt-get &> /dev/null; then
        echo "📦 Installing Ubuntu/Debian dependencies..."
        sudo apt-get update
        sudo apt-get install -y \
            libwebkit2gtk-4.0-dev \
            build-essential \
            curl \
            wget \
            libssl-dev \
            libgtk-3-dev \
            libayatana-appindicator3-dev \
            librsvg2-dev \
            libsoup2.4-dev \
            javascriptcoregtk-4.0 \
            libjavascriptcoregtk-4.0-dev

    # Check if running on CentOS/RHEL/Fedora
    elif command -v dnf &> /dev/null || command -v yum &> /dev/null; then
        echo "📦 Installing CentOS/RHEL/Fedora dependencies..."
        if command -v dnf &> /dev/null; then
            sudo dnf install -y \
                webkit2gtk3-devel \
                openssl-devel \
                curl \
                wget \
                libappindicator-gtk3-devel \
                librsvg2-devel
        else
            sudo yum install -y \
                webkit2gtk3-devel \
                openssl-devel \
                curl \
                wget \
                libappindicator-gtk3-devel \
                librsvg2-devel
        fi

    # Check if running on Arch Linux
    elif command -v pacman &> /dev/null; then
        echo "📦 Installing Arch Linux dependencies..."
        sudo pacman -S --needed \
            webkit2gtk \
            base-devel \
            curl \
            wget \
            openssl \
            appmenu-gtk-module \
            gtk3 \
            libappindicator-gtk3 \
            librsvg
    else
        echo "⚠️  Unknown Linux distribution. Please install Tauri dependencies manually:"
        echo "   https://tauri.app/v1/guides/getting-started/prerequisites#setting-up-linux"
    fi

    echo "✅ Linux dependencies installed"
fi

# Install npm dependencies
echo "📥 Installing frontend dependencies..."
npm install

echo "📥 Installing Tauri dependencies..."
cd src-tauri
cargo fetch
cd ..

# Create build directory
mkdir -p dist

# Build modes
BUILD_MODE=${1:-"dev"}
BUILD_TARGETS=${2:-"current"}

case $BUILD_MODE in
    "dev"|"development")
        echo "🔧 Building in development mode..."
        npm run tauri-dev &
        echo "🌐 Development server started"
        echo "   Frontend: http://localhost:3000"
        echo "   Press Ctrl+C to stop"
        wait
        ;;

    "build"|"release")
        echo "🏗️  Building for production..."

        case $BUILD_TARGETS in
            "current")
                echo "🎯 Building for current platform..."
                npm run tauri-build
                ;;

            "all")
                echo "🌍 Building for all platforms..."

                # Linux build
                if [[ "$OSTYPE" == "linux-gnu"* ]]; then
                    echo "🐧 Building Linux version..."
                    npm run tauri-build
                fi

                # Windows build (cross-compilation) - requires special handling
                echo "🪟 Building Windows version..."
                rustup target add x86_64-pc-windows-gnu

                # Create a temporary tauri config for Windows builds
                echo "📝 Creating Windows-specific build configuration..."
                cp src-tauri/tauri.conf.json src-tauri/tauri.conf.json.backup

                # Update config for Windows targets temporarily
                sed -i 's/"targets": \["deb", "appimage"\]/"targets": ["msi", "nsis"]/' src-tauri/tauri.conf.json

                # Attempt Windows build with error handling
                if npm run tauri-build -- --target x86_64-pc-windows-gnu; then
                    echo "✅ Windows build completed successfully"
                else
                    echo "⚠️  Windows build failed (likely due to cross-compilation limitations)"
                    echo "   Consider building on a Windows host for full Windows support"
                fi

                # Restore original config
                mv src-tauri/tauri.conf.json.backup src-tauri/tauri.conf.json

                # macOS build (if on macOS)
                if [[ "$OSTYPE" == "darwin"* ]]; then
                    echo "🍎 Building macOS version..."
                    npm run tauri-build -- --target x86_64-apple-darwin
                    npm run tauri-build -- --target aarch64-apple-darwin
                fi
                ;;

            "windows")
                echo "🪟 Building for Windows..."
                rustup target add x86_64-pc-windows-gnu
                npm run tauri-build -- --target x86_64-pc-windows-gnu
                ;;

            "linux")
                echo "🐧 Building for Linux..."
                npm run tauri-build -- --target x86_64-unknown-linux-gnu
                ;;

            "macos")
                if [[ "$OSTYPE" == "darwin"* ]]; then
                    echo "🍎 Building for macOS..."
                    npm run tauri-build -- --target x86_64-apple-darwin
                    npm run tauri-build -- --target aarch64-apple-darwin
                else
                    echo "❌ macOS builds can only be created on macOS"
                    exit 1
                fi
                ;;

            *)
                echo "❌ Unknown target: $BUILD_TARGETS"
                echo "   Valid targets: current, all, windows, linux, macos"
                exit 1
                ;;
        esac
        ;;

    "debug")
        echo "🐛 Building debug version..."
        npm run tauri-build-debug
        ;;

    "clean")
        echo "🧹 Cleaning build artifacts..."
        rm -rf dist/
        rm -rf node_modules/
        rm -rf src-tauri/target/
        npm run clean
        echo "✅ Clean complete"
        ;;

    *)
        echo "❌ Unknown build mode: $BUILD_MODE"
        echo ""
        echo "Usage: $0 [MODE] [TARGETS]"
        echo ""
        echo "Modes:"
        echo "  dev        - Start development server"
        echo "  build      - Build for production"
        echo "  debug      - Build debug version"
        echo "  clean      - Clean all build artifacts"
        echo ""
        echo "Targets (for build mode):"
        echo "  current    - Build for current platform (default)"
        echo "  all        - Build for all supported platforms"
        echo "  windows    - Build for Windows"
        echo "  linux      - Build for Linux"
        echo "  macos      - Build for macOS (macOS only)"
        echo ""
        echo "Examples:"
        echo "  $0 dev                    # Start development server"
        echo "  $0 build current          # Build for current platform"
        echo "  $0 build all              # Build for all platforms"
        echo "  $0 build windows          # Build for Windows only"
        echo "  $0 clean                  # Clean build artifacts"
        exit 1
        ;;
esac

# Show build results
if [ "$BUILD_MODE" == "build" ] || [ "$BUILD_MODE" == "debug" ]; then
    echo ""
    echo "🎉 Build completed!"
    echo "📁 Build artifacts:"

    if [ -d "src-tauri/target/release/bundle" ]; then
        find src-tauri/target/release/bundle -name "*.deb" -o -name "*.rpm" -o -name "*.AppImage" -o -name "*.dmg" -o -name "*.msi" -o -name "*.exe" | while read -r file; do
            size=$(du -h "$file" | cut -f1)
            echo "   📦 $(basename "$file") ($size)"
        done
    fi

    echo ""
    echo "🚀 Installation:"
    echo "   Linux:   Install the .deb, .rpm, or .AppImage file"
    echo "   Windows: Run the .msi installer or .exe"
    echo "   macOS:   Mount the .dmg and copy to Applications"
    echo ""
    echo "✅ Nexus C2 Client build complete!"
fi
