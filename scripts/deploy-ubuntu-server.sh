#!/bin/bash

# Ubuntu 22.04 Server Deployment Script for Rust-Nexus C2
# This script automates the deployment of the Nexus server on Ubuntu 22.04

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

# Configuration
NEXUS_USER="nexus"
NEXUS_HOME="/opt/nexus"
SERVICE_NAME="nexus-server"
LOG_DIR="/var/log/nexus"
CONFIG_DIR="/etc/nexus"

echo -e "${BLUE}╔══════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║        Rust-Nexus C2 Ubuntu 22.04 Deployment    ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════╝${NC}"
echo ""

# Check if running as root
if [[ $EUID -ne 0 ]]; then
    echo -e "${RED}❌ This script must be run as root${NC}"
    echo "Please run: sudo $0"
    exit 1
fi

# Verify Ubuntu version
if ! grep -q "Ubuntu 22.04" /etc/os-release; then
    echo -e "${YELLOW}⚠️  Warning: This script is designed for Ubuntu 22.04${NC}"
    read -p "Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Update system packages
echo -e "${BLUE}📦 Updating system packages...${NC}"
apt update -y
apt upgrade -y

# Install essential packages
echo -e "${BLUE}📋 Installing essential packages...${NC}"
apt install -y \
    curl \
    wget \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    ca-certificates \
    gnupg \
    lsb-release \
    ufw \
    fail2ban \
    htop \
    tree \
    jq \
    unzip

# Install Rust toolchain
echo -e "${BLUE}🦀 Installing Rust toolchain...${NC}"
if ! command -v rustc &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env

    # Add rustup to PATH for all users
    echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> /etc/bash.bashrc

    # Install additional targets for cross-compilation
    rustup target add x86_64-pc-windows-gnu
    rustup target add x86_64-unknown-linux-gnu

    echo -e "${GREEN}✅ Rust toolchain installed${NC}"
else
    echo -e "${GREEN}✅ Rust toolchain already installed${NC}"
fi

# Create nexus user
echo -e "${BLUE}👤 Creating nexus user...${NC}"
if ! id "$NEXUS_USER" &>/dev/null; then
    useradd -r -s /bin/bash -d "$NEXUS_HOME" "$NEXUS_USER"
    echo -e "${GREEN}✅ User '$NEXUS_USER' created${NC}"
else
    echo -e "${GREEN}✅ User '$NEXUS_USER' already exists${NC}"
fi

# Create directories
echo -e "${BLUE}📁 Creating directory structure...${NC}"
mkdir -p "$NEXUS_HOME"/{bin,config,certs,logs,data}
mkdir -p "$CONFIG_DIR"
mkdir -p "$LOG_DIR"

# Set permissions
chown -R "$NEXUS_USER:$NEXUS_USER" "$NEXUS_HOME"
chown -R "$NEXUS_USER:$NEXUS_USER" "$LOG_DIR"
chmod 755 "$NEXUS_HOME"
chmod 750 "$CONFIG_DIR"
chmod 750 "$LOG_DIR"

# Configure firewall
echo -e "${BLUE}🔥 Configuring UFW firewall...${NC}"
ufw --force enable
ufw default deny incoming
ufw default allow outgoing
ufw allow ssh
ufw allow 8443/tcp comment "Nexus C2 Server"
ufw allow 443/tcp comment "HTTPS"
ufw allow 80/tcp comment "HTTP (for Let's Encrypt)"

# Configure fail2ban
echo -e "${BLUE}🛡️  Configuring fail2ban...${NC}"
cat > /etc/fail2ban/jail.local << EOF
[DEFAULT]
bantime = 3600
findtime = 600
maxretry = 3

[sshd]
enabled = true
port = ssh
filter = sshd
logpath = /var/log/auth.log
maxretry = 3

[nexus-c2]
enabled = true
port = 8443
filter = nexus-c2
logpath = $LOG_DIR/nexus.log
maxretry = 5
EOF

# Create fail2ban filter for Nexus
cat > /etc/fail2ban/filter.d/nexus-c2.conf << EOF
[Definition]
failregex = ^.* \[.*\] .*: Authentication failed from <HOST>.*$
            ^.* \[.*\] .*: Invalid agent registration from <HOST>.*$
            ^.* \[.*\] .*: Suspicious activity from <HOST>.*$
ignoreregex =
EOF

systemctl restart fail2ban
systemctl enable fail2ban

# Install Docker (optional, for containerized deployment)
echo -e "${BLUE}🐳 Installing Docker (optional)...${NC}"
if ! command -v docker &> /dev/null; then
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null
    apt update
    apt install -y docker-ce docker-ce-cli containerd.io
    usermod -aG docker "$NEXUS_USER"
    systemctl enable docker
    echo -e "${GREEN}✅ Docker installed${NC}"
else
    echo -e "${GREEN}✅ Docker already installed${NC}"
fi

# Create systemd service
echo -e "${BLUE}⚙️  Creating systemd service...${NC}"
cat > "/etc/systemd/system/$SERVICE_NAME.service" << EOF
[Unit]
Description=Rust-Nexus C2 Server
Documentation=https://github.com/cmndcntrlcyber/rust-nexus
After=network.target
Wants=network.target

[Service]
Type=exec
User=$NEXUS_USER
Group=$NEXUS_USER
WorkingDirectory=$NEXUS_HOME
ExecStart=$NEXUS_HOME/bin/nexus-server --config $CONFIG_DIR/nexus.toml
ExecReload=/bin/kill -HUP \$MAINPID
KillMode=mixed
KillSignal=SIGTERM
RestartSec=5
Restart=always
StandardOutput=journal
StandardError=journal
SyslogIdentifier=nexus-server

# Security settings
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ReadWritePaths=$NEXUS_HOME $LOG_DIR $CONFIG_DIR
ProtectHome=yes
ProtectKernelModules=yes
ProtectKernelTunables=yes
ProtectControlGroups=yes
RestrictSUIDSGID=yes
RestrictRealtime=yes
RestrictNamespaces=yes
LockPersonality=yes
MemoryDenyWriteExecute=yes
SystemCallFilter=@system-service
SystemCallErrorNumber=EPERM

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable "$SERVICE_NAME"

# Create log rotation configuration
echo -e "${BLUE}📝 Configuring log rotation...${NC}"
cat > "/etc/logrotate.d/nexus" << EOF
$LOG_DIR/*.log {
    daily
    missingok
    rotate 30
    compress
    delaycompress
    notifempty
    sharedscripts
    postrotate
        systemctl reload $SERVICE_NAME >/dev/null 2>&1 || true
    endscript
    su $NEXUS_USER $NEXUS_USER
}
EOF

# Create example configuration
echo -e "${BLUE}📄 Creating example configuration...${NC}"
cat > "$CONFIG_DIR/nexus.toml.example" << EOF
# Rust-Nexus Server Configuration for Ubuntu 22.04
# Copy this file to nexus.toml and customize

[cloudflare]
api_token = "YOUR_CLOUDFLARE_API_TOKEN_HERE"
zone_id = "YOUR_ZONE_ID_HERE"
domain = "your-domain.com"
proxy_enabled = true

[letsencrypt]
contact_email = "your-email@example.com"
challenge_type = "Dns01"
acme_directory_url = "https://acme-v02.api.letsencrypt.org/directory"
cert_storage_dir = "$NEXUS_HOME/certs"

[grpc_server]
bind_address = "0.0.0.0"
port = 8443
mutual_tls = true
max_connections = 1000

[domains]
primary_domains = [
    "c2.your-domain.com",
    "api.your-domain.com"
]

[logging]
level = "info"
file_output = "$LOG_DIR/nexus.log"
console_output = false
structured = true
EOF

chown "$NEXUS_USER:$NEXUS_USER" "$CONFIG_DIR/nexus.toml.example"

# Create management scripts
echo -e "${BLUE}🔧 Creating management scripts...${NC}"

# Server management script
cat > "$NEXUS_HOME/bin/nexus-ctl" << 'EOF'
#!/bin/bash

SERVICE_NAME="nexus-server"
NEXUS_HOME="/opt/nexus"
LOG_DIR="/var/log/nexus"

case "$1" in
    start)
        echo "Starting Nexus server..."
        sudo systemctl start $SERVICE_NAME
        ;;
    stop)
        echo "Stopping Nexus server..."
        sudo systemctl stop $SERVICE_NAME
        ;;
    restart)
        echo "Restarting Nexus server..."
        sudo systemctl restart $SERVICE_NAME
        ;;
    status)
        sudo systemctl status $SERVICE_NAME
        ;;
    logs)
        sudo journalctl -u $SERVICE_NAME -f
        ;;
    tail)
        sudo tail -f $LOG_DIR/nexus.log
        ;;
    config)
        sudo nano /etc/nexus/nexus.toml
        ;;
    agents)
        echo "Connected agents:"
        # TODO: Implement agent listing
        ;;
    *)
        echo "Usage: $0 {start|stop|restart|status|logs|tail|config|agents}"
        exit 1
        ;;
esac
EOF

chmod +x "$NEXUS_HOME/bin/nexus-ctl"

# Backup script
cat > "$NEXUS_HOME/bin/backup-nexus" << EOF
#!/bin/bash

BACKUP_DIR="$NEXUS_HOME/backups"
DATE=\$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="nexus_backup_\$DATE.tar.gz"

mkdir -p "\$BACKUP_DIR"

echo "Creating backup: \$BACKUP_FILE"
tar -czf "\$BACKUP_DIR/\$BACKUP_FILE" \\
    -C /etc nexus \\
    -C $NEXUS_HOME certs config data logs

echo "Backup created: \$BACKUP_DIR/\$BACKUP_FILE"

# Keep only last 10 backups
cd "\$BACKUP_DIR"
ls -t nexus_backup_*.tar.gz | tail -n +11 | xargs -r rm
EOF

chmod +x "$NEXUS_HOME/bin/backup-nexus"
chown -R "$NEXUS_USER:$NEXUS_USER" "$NEXUS_HOME/bin"

# Create update script
cat > "$NEXUS_HOME/bin/update-nexus" << EOF
#!/bin/bash

NEXUS_REPO="https://github.com/cmndcntrlcyber/rust-nexus.git"
BUILD_DIR="/tmp/nexus-build"

echo "Updating Rust-Nexus C2 server..."

# Create backup before update
$NEXUS_HOME/bin/backup-nexus

# Stop service
sudo systemctl stop $SERVICE_NAME

# Clone latest version
rm -rf "\$BUILD_DIR"
git clone "\$NEXUS_REPO" "\$BUILD_DIR"
cd "\$BUILD_DIR"

# Build server
cargo build --release --bin nexus-server

# Install new binary
sudo cp target/release/nexus-server $NEXUS_HOME/bin/
sudo chown $NEXUS_USER:$NEXUS_USER $NEXUS_HOME/bin/nexus-server
sudo chmod +x $NEXUS_HOME/bin/nexus-server

# Start service
sudo systemctl start $SERVICE_NAME

echo "Update completed!"
EOF

chmod +x "$NEXUS_HOME/bin/update-nexus"

# Set up automatic updates (optional)
echo -e "${BLUE}🔄 Setting up automatic security updates...${NC}"
apt install -y unattended-upgrades
dpkg-reconfigure -plow unattended-upgrades

# Create monitoring script
cat > "$NEXUS_HOME/bin/monitor-nexus" << EOF
#!/bin/bash

echo "=== Nexus Server Status ==="
systemctl is-active $SERVICE_NAME
echo

echo "=== Resource Usage ==="
ps aux | grep nexus-server | grep -v grep
echo

echo "=== Network Connections ==="
ss -tlnp | grep :8443
echo

echo "=== Recent Logs ==="
journalctl -u $SERVICE_NAME --no-pager -n 10
echo

echo "=== Disk Usage ==="
df -h $NEXUS_HOME
echo

echo "=== Memory Usage ==="
free -h
EOF

chmod +x "$NEXUS_HOME/bin/monitor-nexus"

# Final setup
echo -e "${BLUE}🔐 Setting final permissions...${NC}"
chown -R "$NEXUS_USER:$NEXUS_USER" "$NEXUS_HOME"
chmod -R 750 "$NEXUS_HOME"
chmod +x "$NEXUS_HOME/bin/"*

# Summary
echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║           Deployment Completed Successfully!       ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${BLUE}📋 Next Steps:${NC}"
echo "1. Copy your Nexus binary to: $NEXUS_HOME/bin/nexus-server"
echo "2. Configure the server: cp $CONFIG_DIR/nexus.toml.example $CONFIG_DIR/nexus.toml"
echo "3. Edit configuration: sudo nano $CONFIG_DIR/nexus.toml"
echo "4. Start the server: sudo systemctl start $SERVICE_NAME"
echo "5. Check status: sudo systemctl status $SERVICE_NAME"
echo ""
echo -e "${BLUE}🛠️  Management Commands:${NC}"
echo "• $NEXUS_HOME/bin/nexus-ctl {start|stop|restart|status|logs}"
echo "• $NEXUS_HOME/bin/monitor-nexus"
echo "• $NEXUS_HOME/bin/backup-nexus"
echo "• $NEXUS_HOME/bin/update-nexus"
echo ""
echo -e "${BLUE}🔒 Security Notes:${NC}"
echo "• UFW firewall configured with essential ports"
echo "• Fail2ban configured for SSH and Nexus protection"
echo "• Service runs with restricted permissions"
echo "• Automatic security updates enabled"
echo ""
echo -e "${YELLOW}⚠️  Important:${NC}"
echo "• Configure your domain and certificates in $CONFIG_DIR/nexus.toml"
echo "• Set up proper DNS records for your C2 domains"
echo "• Consider using Let's Encrypt for automatic certificate management"
echo "• Monitor logs regularly: journalctl -u $SERVICE_NAME -f"
echo ""
echo -e "${PURPLE}🎉 Ubuntu 22.04 server is ready for Rust-Nexus C2!${NC}"
EOF
<task_progress>
- [x] Analyze current nexus-agent structure
- [x] Examine build configurations and scripts
- [x] Design platform-specific compilation strategy
- [x] Analyze server deployment requirements
- [x] Create agent-linux.toml configuration
- [x] Create agent-windows.toml configuration
- [x] Create Ubuntu 22.04 deployment scripts
- [ ] Update build system for cross-platform support
- [ ] Implement conditional compilation features
- [ ] Create platform-specific build scripts
- [ ] Ensure Ubuntu 22.04 server compatibility
- [ ] Verify cross-platform agent-server connectivity
- [ ] Update documentation
</task_progress>
</write_to_file>
