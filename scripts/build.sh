#!/bin/bash

# RustNexus + FiberWeaver Enhanced Build Script
# Cross-platform build automation with platform-specific configurations

set -e

echo "🚀 Building RustNexus + FiberWeaver C2 Framework"
echo "================================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

# Configuration
BUILD_TYPE="${1:-release}"
PLATFORM="${2:-all}"
OUTPUT_DIR="target/builds"
CONFIG_DIR="config"

echo -e "${BLUE}Build Configuration:${NC}"
echo -e "  Build Type: ${BUILD_TYPE}"
echo -e "  Platform: ${PLATFORM}"
echo -e "  Output Directory: ${OUTPUT_DIR}"
echo ""

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Rust/Cargo not found. Please install Rust first.${NC}"
    echo "Install from: https://rustup.rs/"
    exit 1
fi

echo -e "${GREEN}✅ Rust toolchain found${NC}"

# Check for platform-specific configurations
check_config_files() {
    echo -e "${BLUE}🔧 Checking platform-specific configurations...${NC}"

    if [ -f "${CONFIG_DIR}/agent-linux.toml" ]; then
        echo -e "${GREEN}✅ Linux agent configuration found${NC}"
    else
        echo -e "${YELLOW}⚠️  Linux agent configuration missing${NC}"
    fi

    if [ -f "${CONFIG_DIR}/agent-windows.toml" ]; then
        echo -e "${GREEN}✅ Windows agent configuration found${NC}"
    else
        echo -e "${YELLOW}⚠️  Windows agent configuration missing${NC}"
    fi
}

check_config_files

# Create target directory
mkdir -p target/builds

# Build common library first
echo -e "${BLUE}🔧 Building nexus-common...${NC}"
cd nexus-common
cargo build --release
cd ..

# Build platform-specific agents
echo -e "${BLUE}🔧 Building platform-specific nexus-agents...${NC}"
cd nexus-agent

# Build Linux agent
if cargo build --release --bin nexus-agent-linux; then
    echo -e "${GREEN}✅ Successfully built Linux agent${NC}"
    cp target/release/nexus-agent-linux ../target/builds/ 2>/dev/null || true
else
    echo -e "${RED}❌ Failed to build Linux agent${NC}"
fi

# Build Windows agent if cross-compilation target is available
if rustup target list --installed | grep -q "x86_64-pc-windows-gnu"; then
    if cargo build --release --bin nexus-agent-windows --target x86_64-pc-windows-gnu; then
        echo -e "${GREEN}✅ Successfully built Windows agent${NC}"
        cp target/x86_64-pc-windows-gnu/release/nexus-agent-windows.exe ../target/builds/ 2>/dev/null || true
    else
        echo -e "${RED}❌ Failed to build Windows agent${NC}"
    fi
fi

cd ..

# Cross-compilation targets
TARGETS=()

# Check for Windows cross-compilation
if rustup target list --installed | grep -q "x86_64-pc-windows-gnu"; then
    TARGETS+=("x86_64-pc-windows-gnu")
    echo -e "${GREEN}✅ Windows cross-compilation target available${NC}"
else
    echo -e "${YELLOW}⚠️  Windows cross-compilation not available${NC}"
    echo "To enable: rustup target add x86_64-pc-windows-gnu"
fi

# Check for Linux cross-compilation (from Windows/macOS)
if rustup target list --installed | grep -q "x86_64-unknown-linux-gnu"; then
    TARGETS+=("x86_64-unknown-linux-gnu")
    echo -e "${GREEN}✅ Linux cross-compilation target available${NC}"
else
    echo -e "${YELLOW}⚠️  Linux cross-compilation not available${NC}"
    echo "To enable: rustup target add x86_64-unknown-linux-gnu"
fi

# Enhanced platform-specific build function
build_platform_agent() {
    local target="$1"
    local platform_name="$2"
    local config_file="$3"

    echo -e "${BLUE}🔧 Building nexus-agent for ${target} (${platform_name})...${NC}"
    cd nexus-agent

    # Set platform-specific features
    local features=""
    if [[ "$target" == *"windows"* ]]; then
        features="--features windows-specific,bof-loading,wmi-execution"
    elif [[ "$target" == *"linux"* ]]; then
        features="--features linux-specific,elf-loading,systemd-integration"
    fi

    # Build with platform-specific features
    local build_cmd="cargo build --release --target $target $features"

    if eval $build_cmd 2>/dev/null; then
        echo -e "${GREEN}✅ Successfully built ${platform_name} agent for ${target}${NC}"

        # Copy binaries with platform-specific naming
        if [[ "$target" == *"windows"* ]]; then
            local binary_name="nexus-agent-windows-${target#*-}.exe"
            cp "target/${target}/release/nexus-agent.exe" "../${OUTPUT_DIR}/${binary_name}" 2>/dev/null || true

            # Copy platform-specific config if available
            if [ -f "../${CONFIG_DIR}/agent-windows.toml" ]; then
                cp "../${CONFIG_DIR}/agent-windows.toml" "../${OUTPUT_DIR}/agent-windows-${target#*-}.toml"
            fi
        else
            local binary_name="nexus-agent-linux-${target#*-}"
            cp "target/${target}/release/nexus-agent" "../${OUTPUT_DIR}/${binary_name}" 2>/dev/null || true

            # Copy platform-specific config if available
            if [ -f "../${CONFIG_DIR}/agent-linux.toml" ]; then
                cp "../${CONFIG_DIR}/agent-linux.toml" "../${OUTPUT_DIR}/agent-linux-${target#*-}.toml"
            fi
        fi

        return 0
    else
        echo -e "${RED}❌ Failed to build ${platform_name} agent for ${target}${NC}"
        return 1
    fi

    cd ..
}

# Build server component
build_server() {
    echo -e "${BLUE}🔧 Building nexus-server...${NC}"
    cd nexus-server

    if cargo build --release; then
        echo -e "${GREEN}✅ Successfully built nexus-server${NC}"
        cp target/release/nexus-server "../${OUTPUT_DIR}/nexus-server" 2>/dev/null || true
    else
        echo -e "${RED}❌ Failed to build nexus-server${NC}"
        return 1
    fi

    cd ..
}

# Build based on platform selection
if [ "$PLATFORM" = "all" ] || [ "$PLATFORM" = "server" ]; then
    build_server
fi

# Copy configuration files to build output
if [ "$PLATFORM" = "all" ] || [ "$PLATFORM" = "agents" ]; then
    echo -e "${BLUE}🔧 Copying configuration files...${NC}"

    # Copy Linux agent configuration
    if [ -f "${CONFIG_DIR}/agent-linux.toml.example" ]; then
        cp "${CONFIG_DIR}/agent-linux.toml.example" "${OUTPUT_DIR}/agent-linux.toml.example"
        echo -e "${GREEN}✅ Copied Linux agent configuration${NC}"
    fi

    # Copy Windows agent configuration
    if [ -f "${CONFIG_DIR}/agent-windows.toml.example" ]; then
        cp "${CONFIG_DIR}/agent-windows.toml.example" "${OUTPUT_DIR}/agent-windows.toml.example"
        echo -e "${GREEN}✅ Copied Windows agent configuration${NC}"
    fi
fi

# Create build info
echo -e "${BLUE}📝 Creating build information...${NC}"
cat > target/builds/BUILD_INFO.txt << EOF
RustNexus + FiberWeaver C2 Framework
Build Date: $(date)
Built by: $(whoami)
Host System: $(uname -a)

Available Binaries:
EOF

ls -la target/builds/nexus-agent* >> target/builds/BUILD_INFO.txt 2>/dev/null || echo "No agent binaries found" >> target/builds/BUILD_INFO.txt

# Size optimization (optional)
if command -v strip &> /dev/null; then
    echo -e "${BLUE}🗜️  Stripping debug symbols for size optimization...${NC}"
    find target/builds -name "nexus-agent*" -type f -exec strip {} \; 2>/dev/null || true
    echo -e "${GREEN}✅ Debug symbols stripped${NC}"
fi

# UPX compression (optional)
if command -v upx &> /dev/null; then
    echo -e "${BLUE}📦 Compressing binaries with UPX...${NC}"
    find target/builds -name "nexus-agent*" -type f -exec upx --ultra-brute {} \; 2>/dev/null || true
    echo -e "${GREEN}✅ Binaries compressed${NC}"
else
    echo -e "${YELLOW}⚠️  UPX not found - skipping compression${NC}"
    echo "Install UPX for binary compression: https://upx.github.io/"
fi

# Final summary
echo -e "${GREEN}🎉 Build completed successfully!${NC}"
echo -e "${BLUE}📁 Binaries available in: target/builds/${NC}"
echo ""
echo "Built targets:"
ls -la target/builds/nexus-agent* 2>/dev/null || echo "No binaries found"

echo ""
echo -e "${YELLOW}📋 Next steps:${NC}"
echo "1. Deploy binaries to target systems"
echo "2. Configure C2 server address in agents"
echo "3. Start nexus-server on your C2 infrastructure"
echo "4. Execute agents on target systems"

echo ""
echo -e "${RED}⚠️  Legal Notice:${NC}"
echo "This framework is for authorized security testing only."
echo "Ensure compliance with applicable laws and regulations."
