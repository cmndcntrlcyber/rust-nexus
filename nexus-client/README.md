# Nexus C2 Client

A modern, cross-platform desktop client for the Nexus C2 Framework, built with Tauri and inspired by Cobalt Strike's interface design.

## Features

### 🖥️ **Modern Interface**
- **Dashboard**: Real-time overview of agents, tasks, and infrastructure
- **Session Management**: Interactive agent sessions with terminal access
- **File Browser**: Remote file management with drag-and-drop support
- **BOF Manager**: Beacon Object File library and execution
- **Team Chat**: Collaborative communication system
- **Infrastructure Controls**: Domain rotation and certificate management

### 🔌 **Backend Integration**
- **gRPC Communication**: High-performance bidirectional streaming
- **WebSocket Updates**: Real-time notifications and status updates
- **REST API**: Complete programmatic access to framework features
- **Certificate Management**: Automated TLS certificate handling

### 🌐 **Cross-Platform Support**
- **Windows**: Native Windows application (.msi installer)
- **Linux**: AppImage, .deb, and .rpm packages
- **macOS**: .dmg installer with Apple Silicon support
- **Portable**: Single-binary distribution with no external dependencies

### 🛡️ **Security Features**
- **Mutual TLS**: Certificate-based authentication
- **Certificate Pinning**: Enhanced security validation
- **Session Encryption**: End-to-end encrypted communications
- **Audit Logging**: Comprehensive operation tracking

## Quick Start

### Prerequisites

- **Node.js 16+** and npm
- **Rust 1.70+** with Cargo
- **Tauri CLI**: `cargo install tauri-cli`

### Development

```bash
# Clone and navigate to client directory
cd nexus-client

# Install dependencies
npm install

# Start development server
./build-client.sh dev
```

### Production Build

```bash
# Build for current platform
./build-client.sh build current

# Build for all platforms
./build-client.sh build all

# Build for specific platform
./build-client.sh build windows
./build-client.sh build linux
./build-client.sh build macos
```

## Architecture

### Frontend Stack
- **HTML5/CSS3**: Modern responsive design
- **Vanilla JavaScript**: Lightweight, no framework dependencies
- **WebSocket**: Real-time communication
- **XTerm.js**: Professional terminal emulation

### Backend Integration
- **Tauri**: Rust-based desktop app framework
- **gRPC**: High-performance RPC communication
- **WebSocket**: Real-time event streaming
- **File System**: Native file operations

### Component Structure

```
nexus-client/
├── src/
│   ├── index.html              # Main application shell
│   ├── styles/
│   │   ├── main.css           # Core styles and layout
│   │   ├── components.css     # UI component styles
│   │   └── themes.css         # Theme definitions
│   └── js/
│       ├── main.js            # Application bootstrap
│       ├── utils/
│       │   ├── api.js         # Tauri API wrapper
│       │   └── websocket.js   # WebSocket manager
│       ├── components/
│       │   ├── terminal.js    # Terminal component
│       │   ├── file-browser.js # File browser
│       │   ├── session-table.js # Agent sessions
│       │   ├── chat.js        # Team chat
│       │   └── bof-manager.js # BOF management
│       └── views/
│           ├── dashboard.js   # Dashboard view
│           └── settings.js    # Configuration
└── src-tauri/
    ├── src/
    │   ├── main.rs            # Tauri application
    │   ├── commands.rs        # Backend commands
    │   ├── state.rs           # Application state
    │   ├── grpc_client.rs     # gRPC integration
    │   └── websocket.rs       # WebSocket client
    ├── Cargo.toml             # Rust dependencies
    └── tauri.conf.json        # Tauri configuration
```

## Configuration

### Configuration File

The client configuration is stored in `nexus-client-config.json` in the same directory as the client executable. You can either:

1. **Configure via Settings UI**: Click the Settings button in the client interface
2. **Create manually**: Create the `nexus-client-config.json` file with the following structure

### Server Connection

Create or modify `nexus-client-config.json` with your Nexus C2 server settings:

```json
{
  "server_endpoint": "your-server.com",
  "server_port": 8443,
  "use_tls": true,
  "cert_path": "/path/to/client.crt",
  "key_path": "/path/to/client.key",
  "ca_cert_path": "/path/to/ca.crt",
  "username": "operator",
  "team_name": "red_team",
  "auto_connect": false,
  "websocket_endpoint": "wss://your-server.com:8443/ws",
  "update_interval_ms": 5000,
  "max_concurrent_tasks": 10,
  "log_level": "info"
}
```

### Configuration Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `server_endpoint` | Nexus C2 server hostname or IP | `127.0.0.1` |
| `server_port` | gRPC server port | `8443` |
| `use_tls` | Enable TLS/SSL encryption | `true` |
| `cert_path` | Client certificate file path | `null` |
| `key_path` | Client private key file path | `null` |
| `ca_cert_path` | CA certificate file path | `null` |
| `username` | Operator username | `operator` |
| `team_name` | Team identifier | `red_team` |
| `auto_connect` | Connect automatically on startup | `false` |
| `websocket_endpoint` | WebSocket URL (optional) | Auto-generated |
| `update_interval_ms` | UI update frequency | `5000` |
| `max_concurrent_tasks` | Task execution limit | `10` |
| `log_level` | Logging verbosity | `info` |

## Usage

### Connecting to Server

1. **Configure Connection**: Click Settings and enter server details
2. **Connect**: Click "Connect to Server" on the dashboard
3. **Verify**: Check connection status in the top navigation

### Agent Interaction

1. **View Agents**: Connected agents appear in the left sidebar
2. **Open Session**: Click an agent to open an interactive terminal
3. **Execute Commands**: Type commands in the terminal
4. **File Operations**: Use the file browser for remote file management

### BOF Management

1. **Import BOFs**: Click "Import BOF" to add new Beacon Object Files
2. **Browse Library**: View available BOFs in the BOF Manager
3. **Execute**: Select a BOF and target agent to execute

### Infrastructure Management

1. **Monitor Domains**: View domain health in the sidebar
2. **Rotate Domains**: Click "Rotate Domain" for immediate rotation
3. **Certificate Status**: Monitor certificate validity and expiration

## Building from Source

### Development Build

```bash
# Install dependencies
npm install

# Start development server
npm run tauri-dev
```

### Release Build

```bash
# Build for production
npm run tauri-build

# Build with debug symbols
npm run tauri-build-debug
```

### Cross-Platform Builds

```bash
# Add Windows target (from Linux/macOS)
rustup target add x86_64-pc-windows-gnu

# Build for Windows
npm run tauri-build -- --target x86_64-pc-windows-gnu

# Build for Linux (native)
npm run tauri-build

# Build for macOS (macOS only)
npm run tauri-build -- --target x86_64-apple-darwin
npm run tauri-build -- --target aarch64-apple-darwin
```

## Troubleshooting

### Common Issues

**Connection Failed**
- Verify server endpoint and port
- Check TLS certificate configuration
- Ensure server is running and accessible

**WebSocket Errors**
- Confirm WebSocket endpoint matches server configuration
- Check firewall settings for WebSocket connections
- Verify TLS/SSL certificate validity

**Build Errors**
- Ensure all dependencies are installed
- Update Rust toolchain: `rustup update`
- Clear build cache: `./build-client.sh clean`

### Development Mode

```bash
# Enable debug logging
RUST_LOG=debug npm run tauri-dev

# Open developer tools
# Press F12 or Ctrl+Shift+I in the application
```

### Performance Optimization

- **Connection Pooling**: Adjust `max_concurrent_tasks` for load
- **Update Intervals**: Increase `update_interval_ms` for slower networks
- **WebSocket Buffer**: Configure server-side buffering for high-frequency updates

## Security Considerations

### Certificate Management
- Use strong TLS certificates for all connections
- Implement certificate pinning for enhanced security
- Regularly rotate certificates and update client configurations

### Network Security
- Deploy with proper firewall configurations
- Use VPN or encrypted tunnels for additional security
- Monitor network traffic for anomalies

### Client Security
- Keep client updated with latest security patches
- Use dedicated systems for C2 operations
- Implement proper access controls and user authentication

## Contributing

### Development Setup

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test thoroughly
5. Submit a pull request

### Code Style

- **Rust**: Follow standard Rust formatting with `cargo fmt`
- **JavaScript**: Use ESLint configuration provided
- **CSS**: Follow BEM methodology for component styling

## License

This project is licensed under the MIT License - see the [LICENSE](../LICENSE) file for details.

## Support

- **Documentation**: See the main [rust-nexus documentation](../README.md)
- **Issues**: Report bugs via GitHub issues
- **Discussions**: Join community discussions for questions and feature requests

---

**Built for Security Professionals | Enterprise-Ready | Cross-Platform**
