#!/bin/bash

# RustNexus + FiberWeaver Build Script
# Cross-platform build automation

set -e

echo "🚀 Building RustNexus + FiberWeaver C2 Framework"
echo "================================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Rust/Cargo not found. Please install Rust first.${NC}"
    echo "Install from: https://rustup.rs/"
    exit 1
fi

echo -e "${GREEN}✅ Rust toolchain found${NC}"

# Create target directory
mkdir -p target/builds

# Build common library first
echo -e "${BLUE}🔧 Building nexus-common...${NC}"
cd nexus-common
cargo build --release
cd ..

# ATT&CK technique features (pass via NEXUS_FEATURES env var)
FEATURES="${NEXUS_FEATURES:-}"
FEATURE_FLAG=""
if [ -n "$FEATURES" ]; then
    FEATURE_FLAG="--features $FEATURES"
    echo -e "${BLUE}ATT&CK techniques enabled: ${FEATURES}${NC}"
fi

# Build agent for current platform
echo -e "${BLUE}🔧 Building nexus-agent for current platform...${NC}"
cd nexus-agent
cargo build --release $FEATURE_FLAG
cp target/release/nexus-agent* ../target/builds/ 2>/dev/null || true
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

# Build for cross-compilation targets
for TARGET in "${TARGETS[@]}"; do
    echo -e "${BLUE}🔧 Building nexus-agent for ${TARGET}...${NC}"
    cd nexus-agent
    
    if cargo build --release --target "$TARGET" $FEATURE_FLAG 2>/dev/null; then
        echo -e "${GREEN}✅ Successfully built for ${TARGET}${NC}"
        
        # Copy binaries to builds directory
        if [[ "$TARGET" == *"windows"* ]]; then
            cp "target/${TARGET}/release/nexus-agent.exe" "../target/builds/nexus-agent-${TARGET}.exe" 2>/dev/null || true
        else
            cp "target/${TARGET}/release/nexus-agent" "../target/builds/nexus-agent-${TARGET}" 2>/dev/null || true
        fi
    else
        echo -e "${RED}❌ Failed to build for ${TARGET}${NC}"
    fi
    
    cd ..
done

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
