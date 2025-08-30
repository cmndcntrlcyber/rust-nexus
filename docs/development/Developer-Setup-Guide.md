# Developer Setup Guide - AI Agent Environment

This guide provides step-by-step instructions for AI agents to set up their development environment for the Rust-Nexus WASM UI project.

## 🎯 **Quick Start Checklist**

For experienced agents, here's the rapid setup sequence:

```bash
# 1. Environment Setup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown
cargo install trunk wasm-bindgen-cli

# 2. Project Setup
git clone https://github.com/cmndcntrlcyber/rust-nexus.git
cd rust-nexus
git checkout -b agent-development-<agent-id>

# 3. WASM Project Initialization
mkdir nexus-wasm-ui && cd nexus-wasm-ui
trunk serve --open  # Start development server

# 4. Verify Environment
cargo test --all
trunk build --release
```

## 🛠️ **Detailed Environment Setup**

### **Step 1: Rust Toolchain Installation**

```bash
# Install Rust via rustup (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify installation
rustc --version  # Should show 1.70+ 
cargo --version

# Install required targets and tools
rustup target add wasm32-unknown-unknown
rustup component add rustfmt clippy
```

### **Step 2: WASM Development Tools**

```bash
# Install Trunk (WASM build tool)
cargo install trunk

# Install wasm-bindgen-cli for JS bindings
cargo install wasm-bindgen-cli

# Install additional development tools
cargo install cargo-watch      # File watching for development
cargo install wasm-pack        # Alternative WASM packaging
cargo install basic-http-server # Simple HTTP server for testing

# Verify WASM tools
trunk --version
wasm-bindgen --version
```

### **Step 3: Development Dependencies**

```bash
# Install Node.js (for additional tooling)
# On Linux/MacOS with NVM:
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install --lts
nvm use --lts

# On Windows: Download from nodejs.org

# Install useful development tools
npm install -g @tailwindcss/cli  # CSS framework
npm install -g sass              # SCSS preprocessing
npm install -g serve             # Static file server
```

### **Step 4: IDE and Editor Configuration**

#### **VS Code Setup (Recommended)**
```bash
# Install VS Code extensions for optimal Rust/WASM development
code --install-extension rust-lang.rust-analyzer
code --install-extension tamasfe.even-better-toml
code --install-extension serayuzgur.crates
code --install-extension vadimcn.vscode-lldb
code --install-extension ms-vscode.webview-ui-toolkit
code --install-extension bradlc.vscode-tailwindcss
```

**VS Code Settings (`settings.json`)**:
```json
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.completion.addCallParentheses": false,
  "rust-analyzer.procMacro.enable": true,
  "rust-analyzer.cargo.buildScripts.enable": true,
  "editor.formatOnSave": true,
  "editor.defaultFormatter": "rust-lang.rust-analyzer",
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer",
    "editor.formatOnSave": true
  }
}
```

#### **Alternative Editors**
- **Neovim**: Install `rust-tools.nvim` and `nvim-lspconfig`
- **Emacs**: Use `rustic-mode` with `lsp-mode`
- **IntelliJ**: Install Rust plugin

## 📁 **Project Structure Setup**

### **Directory Layout**
```
rust-nexus/
├── docs/development/          # This documentation
├── nexus-wasm-ui/            # WASM UI project (create this)
│   ├── src/                  # Rust WASM source code
│   │   ├── lib.rs           # Main WASM entry point
│   │   ├── app.rs           # Root application component
│   │   ├── components/      # Reusable UI components
│   │   ├── services/        # API clients and utilities
│   │   └── utils/           # Helper functions
│   ├── assets/              # Static assets (CSS, images)
│   ├── dist/                # Build output directory
│   ├── tests/               # Integration tests
│   ├── Cargo.toml           # Rust project configuration
│   ├── Trunk.toml           # WASM build configuration
│   └── index.html           # HTML entry point
├── nexus-infra/             # Backend gRPC server
├── nexus-agent/             # C2 agents
└── nexus-common/            # Shared libraries
```

### **Initial WASM Project Setup**

Create the WASM UI project structure:

```bash
# From rust-nexus root directory
mkdir nexus-wasm-ui
cd nexus-wasm-ui

# Initialize Cargo project
cargo init --lib

# Create essential directories
mkdir -p src/components src/services src/utils assets tests
```

**Cargo.toml Configuration**:
```toml
[package]
name = "nexus-wasm-ui"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
# WASM Framework
yew = { version = "0.21", features = ["csr"] }
yew-router = "0.18"

# Web APIs and utilities
web-sys = "0.3"
js-sys = "0.3"
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"

# HTTP and gRPC
gloo = { version = "0.8", features = ["net", "storage", "console"] }
tonic-web = "0.5"
prost = "0.12"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Utilities
uuid = { version = "1.0", features = ["v4", "wee_alloc"] }
chrono = { version = "0.4", features = ["serde"] }

# Logging
log = "0.4"
wasm-logger = "0.2"

[dependencies.web-sys]
version = "0.3"
features = [
  "console",
  "Window",
  "Document",
  "Element",
  "HtmlElement",
  "EventTarget",
  "Event",
  "MouseEvent",
  "KeyboardEvent",
  "WebSocket",
  "MessageEvent",
  "BinaryType",
  "Blob",
  "FileReader",
  "Location",
  "History",
  "Storage",
]

[dev-dependencies]
wasm-bindgen-test = "0.3"

[profile.release]
# Optimize for small binary size
opt-level = "s"
lto = true
```

**Trunk.toml Configuration**:
```toml
[build]
# Build target for WASM
target = "wasm32-unknown-unknown"
# Enable release optimizations
release = true
# Set public URL for assets
public_url = "/nexus-ui/"

[watch]
# Directories to watch for changes
watch = ["src", "assets", "index.html"]
# Directories to ignore
ignore = ["target", "dist", "node_modules", ".git"]

[serve]
# Development server configuration
address = "0.0.0.0"
port = 8080
# Enable CORS for API development
cors = true
```

**index.html Template**:
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Nexus C2 Operator Interface</title>
    <link data-trunk rel="sass" href="assets/styles/main.scss" />
    <meta name="description" content="Enterprise C2 operator interface for rust-nexus framework" />
</head>
<body>
    <div id="nexus-app"></div>
</body>
</html>
```

## 🔧 **Development Workflow**

### **Daily Development Cycle**

```bash
# 1. Start development server (automatic rebuild on changes)
trunk serve --open

# 2. Run tests in watch mode (separate terminal)
cargo watch -x "test --target wasm32-unknown-unknown"

# 3. Format code on save (configured in editor)
# Alternatively, manual formatting:
cargo fmt

# 4. Check code quality
cargo clippy -- -D warnings

# 5. Run full test suite
cargo test --all-targets --all-features
```

### **Git Workflow for Agent Development**

```bash
# Create agent-specific branch
git checkout -b agent-development-<agent-id>

# Regular commit cycle
git add .
git commit -m "<agent-id>: implement component X with feature Y"
git push origin agent-development-<agent-id>

# Create pull request when ready for review
# Pull requests are automatically reviewed by Architecture Lead Agent
```

### **Branch Naming Convention**
- `agent-development-<agent-id>` - Main development branch per agent
- `feature-<agent-id>-<feature-name>` - Specific feature branches
- `bugfix-<agent-id>-<issue-description>` - Bug fix branches
- `integration-<sprint-number>` - Cross-agent integration branches

## 🧪 **Testing Setup**

### **Test Environment Configuration**

```bash
# Install testing utilities
cargo install wasm-pack
rustup target add wasm32-unknown-unknown

# Run WASM-specific tests
wasm-pack test --headless --firefox
wasm-pack test --headless --chrome
```

**Test Configuration in Cargo.toml**:
```toml
[[bin]]
name = "test-runner"
path = "tests/test-runner.rs"

[dev-dependencies]
wasm-bindgen-test = "0.3"
js-sys = "0.3"
# Add testing utilities
futures = "0.3"
tokio-test = "0.4"
```

### **Testing Patterns**

**Component Test Template**:
```rust
use wasm_bindgen_test::*;
use yew::prelude::*;
use crate::components::AgentStatusDisplay;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_agent_status_display() {
    // Test implementation
    let props = AgentStatusProps {
        agent_id: "test-agent".to_string(),
        status: AgentStatus::Online,
    };
    
    // Component testing logic
    assert!(true); // Placeholder
}
```

## 🔄 **Build and Deployment**

### **Development Builds**
```bash
# Development build (debug, fast compilation)
trunk build

# Release build (optimized, smaller size)
trunk build --release

# Build with specific features
trunk build --features "debug-mode"
```

### **Production Build Optimization**
```bash
# Optimized production build
trunk build --release
wasm-opt -Os -o dist/optimized.wasm dist/nexus-wasm-ui.wasm

# Analyze bundle size
wasm-pack build --target web --out-dir pkg
ls -la pkg/ # Check generated file sizes
```

### **Local Testing**
```bash
# Serve built files locally
cd dist
basic-http-server --addr 127.0.0.1:8080

# Or use Python
python -m http.server 8080

# Or use Node.js
npx serve .
```

## 🐛 **Debugging and Troubleshooting**

### **Common Setup Issues**

#### **Rust Toolchain Problems**
```bash
# Update Rust toolchain
rustup update

# Reset toolchain if corrupted
rustup self update
rustup default stable

# Clear cargo cache if build issues persist
cargo clean
rm -rf ~/.cargo/registry
```

#### **WASM Build Issues**
```bash
# Reinstall WASM tools
cargo uninstall trunk wasm-bindgen-cli
cargo install trunk wasm-bindgen-cli

# Clear trunk cache
rm -rf ~/.cache/trunk/

# Verify WASM target is installed
rustup target list --installed | grep wasm32
```

#### **Development Server Issues**
```bash
# Check port availability
lsof -i :8080  # On Unix systems
netstat -an | grep 8080  # On Windows

# Use different port
trunk serve --port 3000

# Clear browser cache and restart
# Chrome: Ctrl+Shift+R (hard refresh)
# Firefox: Ctrl+F5
```

### **Debugging Tools**

#### **Browser Developer Tools**
- **Chrome DevTools**: F12, Console tab for WASM logs
- **Firefox**: F12, enable WASM debugging in about:config
- **Safari**: Developer menu → Web Inspector

#### **Rust-specific Debugging**
```bash
# Enable debug logging
RUST_LOG=debug trunk serve

# Enable detailed error output
RUST_BACKTRACE=1 cargo test

# Profile WASM performance
# Use browser's Performance tab with WASM debugging enabled
```

### **Performance Optimization**

#### **Build Size Optimization**
```rust
// In main.rs or lib.rs
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Optimize panic behavior for smaller size
#[cfg(feature = "console_error_panic_hook")]
pub fn set_panic_hook() {
    console_error_panic_hook::set_once();
}
```

#### **Runtime Performance**
```bash
# Enable optimization flags
export RUSTFLAGS="-C target-cpu=native -C opt-level=3"

# Build with link-time optimization
cargo build --release --config 'profile.release.lto=true'
```

## 📚 **Learning Resources**

### **Essential Reading**
- [Yew Framework Documentation](https://yew.rs/)
- [WASM Bindgen Book](https://rustwasm.github.io/wasm-bindgen/)
- [Rust WASM Book](https://rustwasm.github.io/docs/book/)
- [gRPC-Web Documentation](https://github.com/grpc/grpc-web)

### **Code Examples and References**
- [Yew Examples](https://github.com/yewstack/yew/tree/master/examples)
- [WASM Pack Template](https://github.com/rustwasm/wasm-pack-template)
- [Trunk Examples](https://github.com/thedodd/trunk/tree/master/examples)

### **Agent-Specific Resources**
- [Agent Specifications](agent-coordination/Agent-Specifications.md) - Your role and responsibilities
- [Task Distribution Protocol](agent-coordination/Task-Distribution-Protocol.md) - How tasks are assigned
- [API Integration Guide](API-Integration-Guide.md) - Backend integration patterns

## ✅ **Verification Checklist**

After completing setup, verify everything works:

- [ ] Rust toolchain installed and updated
- [ ] WASM target and tools installed  
- [ ] Project structure created
- [ ] Development server starts without errors
- [ ] Basic WASM build completes successfully
- [ ] Tests run in browser environment
- [ ] Editor/IDE configured with proper extensions
- [ ] Git workflow setup with agent-specific branch
- [ ] Dependencies resolve without conflicts
- [ ] Debug tools accessible and functional

## 🚀 **Next Steps**

Once your environment is set up:

1. **Read Agent Specifications**: Understand your specific role and responsibilities
2. **Check Task Distribution**: Get current task assignments
3. **Review Architecture**: Study the technical architecture and patterns
4. **Start Development**: Begin with your assigned foundation tasks
5. **Join Coordination**: Participate in daily sync and reporting

---

**Version**: 1.0.0  
**Last Updated**: 2025-08-29  
**Maintained By**: Documentation Agent  
**Support**: Refer to troubleshooting section or escalate to Architecture Lead Agent
