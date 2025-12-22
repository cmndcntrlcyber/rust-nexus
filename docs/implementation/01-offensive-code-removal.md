# Offensive Code Removal - Baby Steps Implementation Plan

**Document Version:** 1.0
**Created:** 2025-12-21
**Project:** rust-nexus → gov-nexus Transformation
**Phase:** 1 - Offensive Code Removal

## Executive Summary

This document provides a detailed, step-by-step implementation plan for removing all purely offensive components from the rust-nexus C2 framework as part of its transformation into the gov-nexus compliance platform. Each step is designed to be atomic, verifiable, and reversible if needed.

**Principle:** The Baby Steps™ Methodology - Each action must be the smallest possible meaningful change, with validation after every step.

## Table of Contents

1. [Components to Remove](#components-to-remove)
2. [Dependency Analysis](#dependency-analysis)
3. [Implementation Steps](#implementation-steps)
4. [Validation Procedures](#validation-procedures)
5. [Rollback Procedures](#rollback-procedures)
6. [Audit Trail](#audit-trail)

---

## Components to Remove

### 1. Shellcode Injection Infrastructure
- **File:** `/home/cmndcntrl/code/rust-nexus/nexus-agent/src/fiber_execution.rs`
- **Size:** 17,368 bytes
- **Purpose:** Windows fiber-based shellcode injection, process hollowing, early bird injection
- **Status:** Entire file to be removed

### 2. BOF/COFF Loader
- **File:** `/home/cmndcntrl/code/rust-nexus/nexus-infra/src/bof_loader.rs`
- **Size:** 20,435 bytes
- **Purpose:** Beacon Object File (BOF) and COFF executable loading
- **Status:** Entire file to be removed

### 3. Domain Fronting and Traffic Obfuscation Crate
- **Directory:** `/home/cmndcntrl/code/rust-nexus/nexus-web-comms/`
- **Files:**
  - `src/lib.rs` (22,932 bytes)
  - `src/domain_fronting.rs` (199 bytes)
  - `src/traffic_obfuscation.rs` (875 bytes)
  - `src/http_fallback.rs` (825 bytes)
  - `src/websocket_fallback.rs` (225 bytes)
  - `Cargo.toml` (1,397 bytes)
- **Purpose:** Domain fronting, traffic obfuscation, HTTP/WebSocket fallback channels
- **Status:** Entire crate to be removed

### 4. Keylogger Implementation
- **File:** `/home/cmndcntrl/code/rust-nexus/nexus-agent/src/execution.rs`
- **Lines:** 440-603 (164 lines)
- **Functions:**
  - `execute_keylogger_start()`
  - `execute_keylogger_stop()`
  - `execute_keylogger_status()`
  - `execute_keylogger_flush()`
  - `get_keylogger_data()`
- **Status:** Functions and related state management to be removed

### 5. BOF Source Files
- **Directory:** `/home/cmndcntrl/code/rust-nexus/nexus-agent/bofs/`
- **Subdirectories:**
  - `keylogger/` - Keylogger BOF source code
- **Status:** Entire directory to be removed

### 6. Offensive Task Types in Protocol Definitions
- **File:** `/home/cmndcntrl/code/rust-nexus/nexus-infra/proto/nexus.proto`
- **Lines to Remove:**
  - 24-25: `ExecuteShellcode` and `ExecuteBOF` RPC methods
  - 121-128: Offensive TaskType enums (FIBER_SHELLCODE, BOF_EXECUTION, etc.)
  - 139-140: KEYLOGGER task types
  - 141: CREDENTIAL_HARVESTING task type
  - 226-278: ShellcodeRequest, ShellcodeResponse, BOFRequest, BOFResponse messages
  - 235-243: ShellcodeExecutionMethod enum
  - 260-272: BOFArgument and BOFArgumentType messages
- **Status:** Selective removal of offensive proto definitions

---

## Dependency Analysis

### Direct Dependencies Identified

#### 1. `fiber_execution.rs` Dependencies

**Referenced by:**
- `/home/cmndcntrl/code/rust-nexus/nexus-agent/src/main.rs` (line 15)
  - `mod fiber_execution;` - Module declaration
- `/home/cmndcntrl/code/rust-nexus/nexus-agent/src/execution.rs` (line 6)
  - `use crate::fiber_execution::FiberExecutor;` - Import statement
- `/home/cmndcntrl/code/rust-nexus/nexus-agent/src/execution.rs` (lines 44-45, 54)
  - `fiber_executor: FiberExecutor` - Field in TaskExecutor struct
  - `fiber_executor: FiberExecutor::new()` - Initialization
- `/home/cmndcntrl/code/rust-nexus/nexus-agent/src/execution.rs` (lines 72-74)
  - `"fiber_shellcode"` task handler
  - `"fiber_hollowing"` task handler
  - `"early_bird_injection"` task handler
- `/home/cmndcntrl/code/rust-nexus/nexus-agent/src/agent.rs` (lines 40-43)
  - Capability strings: "fiber_execution", "fiber_hollowing", "early_bird_injection", "apc_injection"
- `/home/cmndcntrl/code/rust-nexus/README.md` - Documentation references
- `/home/cmndcntrl/code/rust-nexus/examples/basic-deployment/README.md` - Example references
- `/home/cmndcntrl/code/rust-nexus/nexus-common/src/agent.rs` - Common type references

#### 2. `bof_loader.rs` Dependencies

**Referenced by:**
- `/home/cmndcntrl/code/rust-nexus/nexus-infra/src/lib.rs` (line 14)
  - `pub mod bof_loader;` - Module declaration
- `/home/cmndcntrl/code/rust-nexus/nexus-infra/src/lib.rs` (line 32)
  - `pub use bof_loader::BOFLoader;` - Re-export
- `/home/cmndcntrl/code/rust-nexus/nexus-infra/src/lib.rs` (line 54-55)
  - `InfraError::BofError` - Error type
- `/home/cmndcntrl/code/rust-nexus/nexus-agent/src/execution.rs` (line 9)
  - `use nexus_infra::bof_loader::{BOFLoader, BofArgument, LoadedBof};` - Import
- `/home/cmndcntrl/code/rust-nexus/nexus-agent/src/execution.rs` (lines 17-19)
  - `KEYLOGGER_BOF_DATA` - Embedded BOF binary
- `/home/cmndcntrl/code/rust-nexus/nexus-agent/src/execution.rs` (lines 22-40)
  - `KeyloggerState` struct using BOFLoader
- `/home/cmndcntrl/code/rust-nexus/CLAUDE.md` - Documentation references
- `/home/cmndcntrl/code/rust-nexus/README.md` - Documentation references
- `/home/cmndcntrl/code/rust-nexus/DOCUMENTATION.md` - Documentation references
- `/home/cmndcntrl/code/rust-nexus/docs/execution/bof-guide.md` - BOF usage guide
- `/home/cmndcntrl/code/rust-nexus/docs/execution/keylogger-guide.md` - Keylogger guide

#### 3. `nexus-web-comms` Crate Dependencies

**Referenced by:**
- `/home/cmndcntrl/code/rust-nexus/Cargo.toml` (line 10)
  - Workspace member declaration
- `/home/cmndcntrl/code/rust-nexus/CLAUDE.md` - Documentation references
- `/home/cmndcntrl/code/rust-nexus/docs/integration/CATCH_TAURI_INTEGRATION.md` - Integration references

**Note:** Currently commented out in other crates due to compilation issues - minimal active dependencies.

#### 4. Keylogger Code Dependencies

**Task Type Handlers in `execution.rs`:**
- Lines 84-91: Task type string matching for keylogger operations
- Lines 22-40: KeyloggerState struct definition
- Lines 47, 56: keylogger_state field in TaskExecutor
- Line 19: KEYLOGGER_BOF_DATA constant
- Lines 440-603: Implementation functions
- Lines 607-619: Helper method `get_keylogger_data()`

#### 5. Workspace Dependencies Affected

**In `/home/cmndcntrl/code/rust-nexus/Cargo.toml`:**
- Lines 58-59: `pelite` and `goblin` - PE/COFF parsing (used by bof_loader)
  - **Decision:** Keep - may be useful for binary analysis in compliance context

**In `/home/cmndcntrl/code/rust-nexus/nexus-agent/Cargo.toml`:**
- Lines 40-41: Feature flags `bof-loading`, `wmi-execution`, etc.
  - **Decision:** Remove offensive feature flags

**In `/home/cmndcntrl/code/rust-nexus/nexus-infra/Cargo.toml`:**
- Lines 57-59: `goblin` and `pelite` dependencies
  - **Decision:** Keep for now, evaluate later if binary analysis is needed

### Proto Definition Impact

Removing proto definitions will require regenerating gRPC code:
- Build script: `/home/cmndcntrl/code/rust-nexus/nexus-infra/build.rs`
- Generated code location: Target directory (auto-regenerated)
- All crates importing `nexus_infra::proto` will need rebuild

---

## Implementation Steps

### Phase 1: Pre-Flight Checks (Steps 1-3)

#### Step 1: Create Backup Branch
**Objective:** Ensure we can rollback if needed

**Actions:**
```bash
cd /home/cmndcntrl/code/rust-nexus
git checkout -b backup/pre-offensive-removal
git push -u origin backup/pre-offensive-removal
git checkout main
git checkout -b feature/remove-offensive-code
```

**Validation:**
- Verify backup branch exists: `git branch --list backup/pre-offensive-removal`
- Verify we're on feature branch: `git branch --show-current`

**Expected Outcome:** Clean backup of current state preserved

---

#### Step 2: Document Current Build State
**Objective:** Establish baseline for comparison

**Actions:**
```bash
# Capture current build status
cargo build --workspace 2>&1 | tee /tmp/build-status-before.log

# Document current tests
cargo test --workspace --no-fail-fast 2>&1 | tee /tmp/test-status-before.log

# List all workspace members
cargo metadata --format-version 1 | jq '.workspace_members' > /tmp/workspace-members-before.json
```

**Validation:**
- Review build log for current state
- Note any existing compilation warnings/errors
- Save logs for comparison

**Expected Outcome:** Baseline documentation of build health

---

#### Step 3: Verify File Existence
**Objective:** Confirm all target files exist before removal

**Actions:**
```bash
# Check each file/directory existence
test -f /home/cmndcntrl/code/rust-nexus/nexus-agent/src/fiber_execution.rs && echo "✓ fiber_execution.rs exists"
test -f /home/cmndcntrl/code/rust-nexus/nexus-infra/src/bof_loader.rs && echo "✓ bof_loader.rs exists"
test -d /home/cmndcntrl/code/rust-nexus/nexus-web-comms && echo "✓ nexus-web-comms/ exists"
test -f /home/cmndcntrl/code/rust-nexus/nexus-agent/src/execution.rs && echo "✓ execution.rs exists"
test -d /home/cmndcntrl/code/rust-nexus/nexus-agent/bofs && echo "✓ bofs/ exists"
test -f /home/cmndcntrl/code/rust-nexus/nexus-infra/proto/nexus.proto && echo "✓ nexus.proto exists"
```

**Validation:**
- All files should exist
- If any file missing, update this plan accordingly

**Expected Outcome:** Confirmation of all target files present

---

### Phase 2: Remove Standalone Files (Steps 4-6)

#### Step 4: Remove fiber_execution.rs
**Objective:** Delete shellcode injection file and remove references

**Substep 4.1: Remove file**
```bash
cd /home/cmndcntrl/code/rust-nexus
git rm nexus-agent/src/fiber_execution.rs
```

**Substep 4.2: Remove module declaration from main.rs**

Edit `/home/cmndcntrl/code/rust-nexus/nexus-agent/src/main.rs`:
- Remove lines 14-15:
  ```rust
  #[cfg(target_os = "windows")]
  mod fiber_execution;
  ```

**Substep 4.3: Remove import from execution.rs**

Edit `/home/cmndcntrl/code/rust-nexus/nexus-agent/src/execution.rs`:
- Remove lines 5-6:
  ```rust
  #[cfg(target_os = "windows")]
  use crate::fiber_execution::FiberExecutor;
  ```

**Substep 4.4: Remove FiberExecutor field from TaskExecutor**

Edit `/home/cmndcntrl/code/rust-nexus/nexus-agent/src/execution.rs`:
- In struct `TaskExecutor` (around line 43-48), remove:
  ```rust
  #[cfg(target_os = "windows")]
  fiber_executor: FiberExecutor,
  ```
- In `TaskExecutor::new()` (around line 50-58), remove:
  ```rust
  #[cfg(target_os = "windows")]
  fiber_executor: FiberExecutor::new(),
  ```

**Substep 4.5: Remove fiber execution task handlers**

Edit `/home/cmndcntrl/code/rust-nexus/nexus-agent/src/execution.rs`:
- In `execute_task()` method (around lines 60-97), remove lines 71-74:
  ```rust
  // Fiber-based execution methods
  "fiber_shellcode" => self.execute_fiber_shellcode(&task_data).await,
  "fiber_hollowing" => self.execute_fiber_hollowing(&task_data).await,
  "early_bird_injection" => self.execute_early_bird_injection(&task_data).await,
  ```

**Substep 4.6: Remove fiber method implementations**

Edit `/home/cmndcntrl/code/rust-nexus/nexus-agent/src/execution.rs`:
- Search for and remove these method implementations (likely after line 600):
  - `async fn execute_fiber_shellcode()`
  - `async fn execute_fiber_hollowing()`
  - `async fn execute_early_bird_injection()`

**Substep 4.7: Remove fiber capabilities from agent.rs**

Edit `/home/cmndcntrl/code/rust-nexus/nexus-agent/src/agent.rs`:
- In the Windows-specific capabilities section (around lines 34-44), remove:
  ```rust
  capabilities.push("fiber_execution".to_string());
  capabilities.push("fiber_hollowing".to_string());
  capabilities.push("early_bird_injection".to_string());
  capabilities.push("apc_injection".to_string());
  ```

**Validation:**
```bash
# Verify file is removed
test ! -f /home/cmndcntrl/code/rust-nexus/nexus-agent/src/fiber_execution.rs

# Verify no remaining references
cd /home/cmndcntrl/code/rust-nexus
grep -r "fiber_execution" --include="*.rs" nexus-agent/src/ || echo "✓ No references found"
grep -r "FiberExecutor" --include="*.rs" nexus-agent/src/ || echo "✓ No FiberExecutor references"
grep -r "fiber_shellcode\|fiber_hollowing\|early_bird" --include="*.rs" nexus-agent/src/ || echo "✓ No fiber task references"

# Attempt compilation
cargo check -p nexus-agent 2>&1 | tee /tmp/step4-build.log
```

**Expected Outcome:**
- File deleted from git
- All references removed
- nexus-agent may still have errors related to keylogger (to be fixed in later steps)
- No fiber-related code remains

**Commit Point:**
```bash
git add -A
git commit -m "Remove fiber_execution.rs and all shellcode injection capabilities

- Deleted nexus-agent/src/fiber_execution.rs (shellcode injection)
- Removed FiberExecutor from TaskExecutor struct
- Removed fiber task handlers: fiber_shellcode, fiber_hollowing, early_bird_injection
- Removed fiber-related capabilities from agent registration
- Part of offensive code removal for gov-nexus transformation"
```

---

#### Step 5: Remove BOF Directory
**Objective:** Delete BOF source files

**Actions:**
```bash
cd /home/cmndcntrl/code/rust-nexus
git rm -r nexus-agent/bofs/
```

**Validation:**
```bash
# Verify directory removed
test ! -d /home/cmndcntrl/code/rust-nexus/nexus-agent/bofs

# Check git status
git status | grep "deleted.*bofs"
```

**Expected Outcome:**
- BOF source directory completely removed
- No impact on build yet (BOF data embedded at compile time)

**Commit Point:**
```bash
git add -A
git commit -m "Remove BOF source files directory

- Deleted nexus-agent/bofs/ directory containing keylogger BOF sources
- Part of offensive code removal for gov-nexus transformation"
```

---

#### Step 6: Remove Keylogger Code from execution.rs
**Objective:** Remove keylogger implementation while preserving file structure

**Substep 6.1: Remove keylogger imports and constants**

Edit `/home/cmndcntrl/code/rust-nexus/nexus-agent/src/execution.rs`:
- Remove lines 8-9:
  ```rust
  #[cfg(target_os = "windows")]
  use nexus_infra::bof_loader::{BOFLoader, BofArgument, LoadedBof};
  ```
- Remove lines 14-15 (after previous removals):
  ```rust
  #[cfg(target_os = "windows")]
  use std::ffi::c_void;
  ```
- Remove lines 17-19:
  ```rust
  #[cfg(target_os = "windows")]
  const KEYLOGGER_BOF_DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/nexus_keylogger.o"));
  ```

**Substep 6.2: Remove KeyloggerState struct**

Edit `/home/cmndcntrl/code/rust-nexus/nexus-agent/src/execution.rs`:
- Remove lines 21-41 (KeyloggerState struct and implementation):
  ```rust
  #[cfg(target_os = "windows")]
  #[derive(Clone)]
  struct KeyloggerState {
      loaded_bof: Option<Arc<LoadedBof>>,
      bof_loader: Arc<BOFLoader>,
      is_active: bool,
      data_buffer: Arc<Mutex<Vec<String>>>,
  }

  #[cfg(target_os = "windows")]
  impl KeyloggerState {
      fn new() -> Self {
          Self {
              loaded_bof: None,
              bof_loader: Arc::new(BOFLoader::new()),
              is_active: false,
              data_buffer: Arc::new(Mutex::new(Vec::new())),
          }
      }
  }
  ```

**Substep 6.3: Remove keylogger_state field from TaskExecutor**

Edit `/home/cmndcntrl/code/rust-nexus/nexus-agent/src/execution.rs`:
- In `TaskExecutor` struct (after previous edits, around line 43-48), remove:
  ```rust
  #[cfg(target_os = "windows")]
  keylogger_state: Arc<Mutex<KeyloggerState>>,
  ```
- In `TaskExecutor::new()`, remove:
  ```rust
  #[cfg(target_os = "windows")]
  keylogger_state: Arc::new(Mutex::new(KeyloggerState::new())),
  ```

**Substep 6.4: Remove keylogger task handlers**

Edit `/home/cmndcntrl/code/rust-nexus/nexus-agent/src/execution.rs`:
- In `execute_task()` method, remove (around lines 83-91 after previous edits):
  ```rust
  // Keylogger BOF operations
  #[cfg(target_os = "windows")]
  "keylogger_start" => self.execute_keylogger_start(&task_data).await,
  #[cfg(target_os = "windows")]
  "keylogger_stop" => self.execute_keylogger_stop(&task_data).await,
  #[cfg(target_os = "windows")]
  "keylogger_status" => self.execute_keylogger_status(&task_data).await,
  #[cfg(target_os = "windows")]
  "keylogger_flush" => self.execute_keylogger_flush(&task_data).await,
  ```

**Substep 6.5: Remove keylogger method implementations**

Edit `/home/cmndcntrl/code/rust-nexus/nexus-agent/src/execution.rs`:
- Remove lines 438-603 (approximately, exact lines after previous edits):
  - `async fn execute_keylogger_start()`
  - `async fn execute_keylogger_stop()`
  - `async fn execute_keylogger_status()`
  - `async fn execute_keylogger_flush()`
- Also remove lines 605-619 (approximately):
  - `pub async fn get_keylogger_data()`

**Validation:**
```bash
# Verify no keylogger references
cd /home/cmndcntrl/code/rust-nexus
grep -n "keylogger" --include="*.rs" nexus-agent/src/execution.rs || echo "✓ No keylogger references"
grep -n "KeyloggerState\|KEYLOGGER_BOF" --include="*.rs" nexus-agent/src/ || echo "✓ No keylogger types"

# Check compilation
cargo check -p nexus-agent 2>&1 | tee /tmp/step6-build.log
```

**Expected Outcome:**
- All keylogger code removed from execution.rs
- File is ~440 lines shorter
- nexus-agent still may not compile due to bof_loader dependency
- No keylogger functionality remains

**Commit Point:**
```bash
git add -A
git commit -m "Remove keylogger implementation from execution.rs

- Removed KeyloggerState struct and management code
- Removed all keylogger task handlers and implementations (lines 440-619)
- Removed KEYLOGGER_BOF_DATA embedded binary
- Removed BOFLoader imports (still used in nexus-infra, to be removed next)
- Part of offensive code removal for gov-nexus transformation"
```

---

### Phase 3: Remove BOF Infrastructure (Steps 7-8)

#### Step 7: Remove bof_loader.rs
**Objective:** Delete BOF/COFF loader and remove references

**Substep 7.1: Remove file**
```bash
cd /home/cmndcntrl/code/rust-nexus
git rm nexus-infra/src/bof_loader.rs
```

**Substep 7.2: Remove module declaration from lib.rs**

Edit `/home/cmndcntrl/code/rust-nexus/nexus-infra/src/lib.rs`:
- Remove line 14:
  ```rust
  pub mod bof_loader;
  ```

**Substep 7.3: Remove re-export**

Edit `/home/cmndcntrl/code/rust-nexus/nexus-infra/src/lib.rs`:
- Remove from line 32:
  ```rust
  pub use bof_loader::BOFLoader;
  ```

**Substep 7.4: Remove BofError variant**

Edit `/home/cmndcntrl/code/rust-nexus/nexus-infra/src/lib.rs`:
- Remove lines 54-55:
  ```rust
  #[error("BOF loading error: {0}")]
  BofError(String),
  ```

**Validation:**
```bash
# Verify file removed
test ! -f /home/cmndcntrl/code/rust-nexus/nexus-infra/src/bof_loader.rs

# Verify no references in nexus-infra
cd /home/cmndcntrl/code/rust-nexus
grep -r "bof_loader\|BOFLoader\|BofError" --include="*.rs" nexus-infra/src/ || echo "✓ No BOF references in nexus-infra"

# Check compilation
cargo check -p nexus-infra 2>&1 | tee /tmp/step7-build.log
```

**Expected Outcome:**
- bof_loader.rs deleted
- nexus-infra compiles successfully
- BOF loading capability completely removed

**Commit Point:**
```bash
git add -A
git commit -m "Remove bof_loader.rs and BOF loading infrastructure

- Deleted nexus-infra/src/bof_loader.rs (BOF/COFF loader)
- Removed BOFLoader re-exports from lib.rs
- Removed BofError from InfraError enum
- Part of offensive code removal for gov-nexus transformation"
```

---

#### Step 8: Remove BOF-Related Feature Flags
**Objective:** Clean up Cargo.toml feature flags

**Substep 8.1: Remove offensive features from nexus-agent**

Edit `/home/cmndcntrl/code/rust-nexus/nexus-agent/Cargo.toml`:
- Remove lines 40-42 (feature definitions):
  ```toml
  # Execution capabilities
  bof-loading = []
  wmi-execution = []
  ```
- Remove lines 52 (offensive features):
  ```toml
  process-injection = []
  ```

**Note:** Keep `elf-loading` and `systemd-integration` as these may be legitimate for Linux compliance tools.

**Substep 8.2: Remove offensive features from nexus-web-comms**
(Will be removed entirely in next phase, but clean for consistency)

Edit `/home/cmndcntrl/code/rust-nexus/nexus-web-comms/Cargo.toml`:
- Note features for removal documentation (entire crate being removed)

**Validation:**
```bash
# Check no BOF features remain
cd /home/cmndcntrl/code/rust-nexus
grep -n "bof-loading\|process-injection" nexus-agent/Cargo.toml || echo "✓ No offensive features"

# Verify workspace still builds
cargo check --workspace 2>&1 | tee /tmp/step8-build.log
```

**Expected Outcome:**
- Offensive feature flags removed
- Workspace may still have issues with nexus-web-comms
- Documentation cleaner

**Commit Point:**
```bash
git add -A
git commit -m "Remove offensive feature flags from Cargo.toml

- Removed bof-loading and process-injection features from nexus-agent
- Cleaned up execution capability feature flags
- Part of offensive code removal for gov-nexus transformation"
```

---

### Phase 4: Remove nexus-web-comms Crate (Steps 9-10)

#### Step 9: Remove nexus-web-comms from Workspace
**Objective:** Delete entire crate and remove from workspace

**Substep 9.1: Remove workspace member**

Edit `/home/cmndcntrl/code/rust-nexus/Cargo.toml`:
- Remove line 10 from workspace members:
  ```toml
  "nexus-web-comms"
  ```

**Substep 9.2: Delete crate directory**
```bash
cd /home/cmndcntrl/code/rust-nexus
git rm -r nexus-web-comms/
```

**Validation:**
```bash
# Verify directory removed
test ! -d /home/cmndcntrl/code/rust-nexus/nexus-web-comms

# Verify workspace members
cargo metadata --format-version 1 | jq '.workspace_members' | grep -v "nexus-web-comms" || echo "✓ Removed from workspace"

# Check workspace builds
cargo check --workspace 2>&1 | tee /tmp/step9-build.log
```

**Expected Outcome:**
- nexus-web-comms completely removed
- Workspace builds successfully
- Domain fronting and traffic obfuscation capabilities gone

**Commit Point:**
```bash
git add -A
git commit -m "Remove nexus-web-comms crate entirely

- Deleted nexus-web-comms/ directory (domain fronting, traffic obfuscation)
- Removed from workspace members in root Cargo.toml
- Removed capabilities: domain fronting, HTTP/WebSocket fallback, traffic obfuscation
- Part of offensive code removal for gov-nexus transformation"
```

---

#### Step 10: Verify No Dangling References
**Objective:** Ensure no code still references removed crate

**Actions:**
```bash
cd /home/cmndcntrl/code/rust-nexus

# Search for imports
grep -r "nexus_web_comms\|nexus-web-comms" --include="*.rs" --include="*.toml" . || echo "✓ No references"

# Search for domain fronting references
grep -r "domain.fronting\|DomainFronting" --include="*.rs" . || echo "✓ No domain fronting code"

# Search for traffic obfuscation
grep -r "traffic.obfuscation\|TrafficObfuscation" --include="*.rs" . || echo "✓ No traffic obfuscation"
```

**Validation:**
```bash
# Full workspace build
cargo build --workspace --all-targets 2>&1 | tee /tmp/step10-build.log

# Check for warnings
cargo clippy --workspace 2>&1 | tee /tmp/step10-clippy.log
```

**Expected Outcome:**
- No references to nexus-web-comms found
- Clean build across all crates
- Ready for proto definition cleanup

**Commit Point:** (Only if issues found)
```bash
git add -A
git commit -m "Clean up remaining nexus-web-comms references

- [Document any additional changes made]
- Part of offensive code removal for gov-nexus transformation"
```

---

### Phase 5: Update Protocol Definitions (Steps 11-13)

#### Step 11: Remove Offensive RPC Methods from Proto
**Objective:** Remove shellcode and BOF RPC definitions

**Substep 11.1: Remove RPC method declarations**

Edit `/home/cmndcntrl/code/rust-nexus/nexus-infra/proto/nexus.proto`:
- Remove lines 23-25 (Advanced execution RPCs):
  ```protobuf
  // Advanced execution
  rpc ExecuteShellcode(ShellcodeRequest) returns (ShellcodeResponse);
  rpc ExecuteBOF(BOFRequest) returns (BOFResponse);
  ```

**Substep 11.2: Document change**
Add comment explaining removal:
```protobuf
// Lines 23-25: Removed ExecuteShellcode and ExecuteBOF RPC methods
// These offensive capabilities removed during gov-nexus transformation
```

**Validation:**
```bash
# Check proto syntax
cd /home/cmndcntrl/code/rust-nexus
protoc --proto_path=nexus-infra/proto --decode_raw < /dev/null nexus-infra/proto/nexus.proto || echo "Note: protoc may not be installed"

# Attempt build (will regenerate proto code)
cargo build -p nexus-infra 2>&1 | tee /tmp/step11-build.log
```

**Expected Outcome:**
- RPC methods removed from service definition
- Proto file still syntactically valid
- Build will regenerate gRPC code

**Commit Point:**
```bash
git add -A
git commit -m "Remove offensive RPC methods from nexus.proto

- Removed ExecuteShellcode and ExecuteBOF RPC methods
- Part of offensive code removal for gov-nexus transformation"
```

---

#### Step 12: Remove Offensive TaskType Enums
**Objective:** Remove offensive task type definitions

**Substep 12.1: Remove advanced execution task types**

Edit `/home/cmndcntrl/code/rust-nexus/nexus-infra/proto/nexus.proto`:
- Remove lines 120-128 (after adjusting for previous deletions):
  ```protobuf
  // Advanced execution methods
  TASK_TYPE_FIBER_SHELLCODE = 20;
  TASK_TYPE_FIBER_HOLLOWING = 21;
  TASK_TYPE_PROCESS_INJECTION = 22;
  TASK_TYPE_DLL_INJECTION = 23;
  TASK_TYPE_APC_INJECTION = 24;
  TASK_TYPE_EARLY_BIRD_INJECTION = 25;
  TASK_TYPE_BOF_EXECUTION = 26;
  TASK_TYPE_COFF_LOADING = 27;
  ```

**Substep 12.2: Remove keylogger task types**

Edit `/home/cmndcntrl/code/rust-nexus/nexus-infra/proto/nexus.proto`:
- In the Reconnaissance section (around line 134-140), remove:
  ```protobuf
  TASK_TYPE_KEYLOGGER_START = 44;
  TASK_TYPE_KEYLOGGER_STOP = 45;
  ```

**Substep 12.3: Remove credential harvesting**

Edit `/home/cmndcntrl/code/rust-nexus/nexus-infra/proto/nexus.proto`:
- In the Reconnaissance section, remove:
  ```protobuf
  TASK_TYPE_CREDENTIAL_HARVESTING = 41;
  ```

**Note:** Keep TASK_TYPE_BROWSER_DATA_EXTRACTION and TASK_TYPE_SCREEN_CAPTURE as these may be legitimate for compliance monitoring (eDiscovery, data leak prevention). Evaluate in later phase.

**Validation:**
```bash
# Check for remaining offensive types
cd /home/cmndcntrl/code/rust-nexus
grep -n "SHELLCODE\|BOF\|INJECTION\|KEYLOGGER\|CREDENTIAL_HARVESTING" nexus-infra/proto/nexus.proto || echo "✓ Offensive types removed"

# Build
cargo build -p nexus-infra 2>&1 | tee /tmp/step12-build.log
```

**Expected Outcome:**
- Offensive task types removed
- Some reconnaissance types retained for evaluation
- Proto regenerates successfully

**Commit Point:**
```bash
git add -A
git commit -m "Remove offensive TaskType enums from nexus.proto

- Removed fiber/shellcode/injection task types (20-27)
- Removed keylogger task types (44-45)
- Removed credential harvesting task type
- Retained browser/screen capture for compliance use case evaluation
- Part of offensive code removal for gov-nexus transformation"
```

---

#### Step 13: Remove Offensive Message Types
**Objective:** Remove shellcode and BOF message definitions

**Substep 13.1: Remove ShellcodeRequest and Response**

Edit `/home/cmndcntrl/code/rust-nexus/nexus-infra/proto/nexus.proto`:
- Remove lines 225-249 (approximately, after previous edits):
  ```protobuf
  // Advanced execution
  message ShellcodeRequest {
    string agent_id = 1;
    bytes shellcode = 2;
    ShellcodeExecutionMethod method = 3;
    string target_process = 4; // For injection methods
    uint32 target_pid = 5;     // For PID-based injection
    map<string, string> options = 6;
  }

  enum ShellcodeExecutionMethod {
    SHELLCODE_EXECUTION_METHOD_UNSPECIFIED = 0;
    SHELLCODE_EXECUTION_METHOD_DIRECT_FIBER = 1;
    SHELLCODE_EXECUTION_METHOD_FIBER_HOLLOWING = 2;
    SHELLCODE_EXECUTION_METHOD_EARLY_BIRD = 3;
    SHELLCODE_EXECUTION_METHOD_PROCESS_INJECTION = 4;
    SHELLCODE_EXECUTION_METHOD_APC_INJECTION = 5;
    SHELLCODE_EXECUTION_METHOD_DLL_INJECTION = 6;
  }

  message ShellcodeResponse {
    bool success = 1;
    string message = 2;
    uint32 process_id = 3; // PID of target process if applicable
  }
  ```

**Substep 13.2: Remove BOFRequest and Response**

Edit `/home/cmndcntrl/code/rust-nexus/nexus-infra/proto/nexus.proto`:
- Remove lines 251-278 (approximately):
  ```protobuf
  // BOF (Beacon Object File) execution
  message BOFRequest {
    string agent_id = 1;
    bytes bof_data = 2;
    string function_name = 3;
    repeated BOFArgument arguments = 4;
    map<string, string> options = 5;
  }

  message BOFArgument {
    BOFArgumentType type = 1;
    bytes value = 2;
  }

  enum BOFArgumentType {
    BOF_ARGUMENT_TYPE_UNSPECIFIED = 0;
    BOF_ARGUMENT_TYPE_INT32 = 1;
    BOF_ARGUMENT_TYPE_INT16 = 2;
    BOF_ARGUMENT_TYPE_STRING = 3;
    BOF_ARGUMENT_TYPE_WSTRING = 4;
    BOF_ARGUMENT_TYPE_BINARY = 5;
  }

  message BOFResponse {
    bool success = 1;
    string message = 2;
    string output = 3;
  }
  ```

**Substep 13.3: Add documentation comment**

Add at the location where messages were removed:
```protobuf
// Advanced execution message types (ShellcodeRequest/Response, BOFRequest/Response)
// removed during gov-nexus transformation - offensive capabilities not needed for compliance
```

**Validation:**
```bash
# Verify message types removed
cd /home/cmndcntrl/code/rust-nexus
grep -n "ShellcodeRequest\|BOFRequest\|ShellcodeExecutionMethod\|BOFArgument" nexus-infra/proto/nexus.proto || echo "✓ Offensive messages removed"

# Full proto validation and rebuild
cargo clean -p nexus-infra
cargo build -p nexus-infra 2>&1 | tee /tmp/step13-build.log
```

**Expected Outcome:**
- All shellcode and BOF message types removed
- Proto file compiles cleanly
- Generated Rust code no longer includes offensive types

**Commit Point:**
```bash
git add -A
git commit -m "Remove offensive message types from nexus.proto

- Removed ShellcodeRequest, ShellcodeResponse, ShellcodeExecutionMethod
- Removed BOFRequest, BOFResponse, BOFArgument, BOFArgumentType
- Proto definitions now aligned with compliance-focused architecture
- Part of offensive code removal for gov-nexus transformation"
```

---

### Phase 6: Cleanup and Documentation (Steps 14-17)

#### Step 14: Remove Documentation for Offensive Features
**Objective:** Remove or update guides for removed features

**Substep 14.1: Remove offensive documentation files**

```bash
cd /home/cmndcntrl/code/rust-nexus

# Remove BOF guide
git rm docs/execution/bof-guide.md

# Remove keylogger guide
git rm docs/execution/keylogger-guide.md
```

**Substep 14.2: Update README files**

Edit `/home/cmndcntrl/code/rust-nexus/README.md`:
- Remove sections describing:
  - Fiber execution capabilities
  - BOF/COFF loading
  - Keylogger functionality
  - Domain fronting
- Add note about transformation to gov-nexus
- Update feature list to reflect compliance focus

Edit `/home/cmndcntrl/code/rust-nexus/DOCUMENTATION.md`:
- Remove references to offensive capabilities
- Update architecture diagrams if present
- Add link to this implementation document

Edit `/home/cmndcntrl/code/rust-nexus/CLAUDE.md`:
- Remove bof_loader references
- Remove nexus-web-comms references
- Update development guidelines

**Substep 14.3: Update example documentation**

Edit `/home/cmndcntrl/code/rust-nexus/examples/basic-deployment/README.md`:
- Remove fiber_execution examples
- Update deployment examples to reflect compliance use cases

**Validation:**
```bash
# Search for remaining offensive documentation
cd /home/cmndcntrl/code/rust-nexus
grep -r "fiber.execution\|bof.loader\|keylogger\|domain.fronting" --include="*.md" docs/ README.md || echo "✓ Documentation cleaned"
```

**Expected Outcome:**
- Offensive capability documentation removed
- README files updated for gov-nexus focus
- No confusing legacy documentation remains

**Commit Point:**
```bash
git add -A
git commit -m "Remove and update documentation for offensive features

- Removed bof-guide.md and keylogger-guide.md
- Updated README.md to reflect gov-nexus compliance focus
- Cleaned offensive capability references from DOCUMENTATION.md and CLAUDE.md
- Updated examples to show legitimate compliance use cases
- Part of offensive code removal for gov-nexus transformation"
```

---

#### Step 15: Review and Remove Unused Dependencies
**Objective:** Remove workspace dependencies only used by offensive code

**Substep 15.1: Evaluate pelite and goblin**

Check if these are still needed:
```bash
cd /home/cmndcntrl/code/rust-nexus
grep -r "pelite\|goblin" --include="*.rs" . || echo "Not used in Rust code"
```

**Decision logic:**
- If only referenced in removed bof_loader.rs → Remove
- If used elsewhere for legitimate binary analysis → Keep and document
- If unsure → Keep for now, mark for Phase 2 evaluation

**Substep 15.2: Update workspace Cargo.toml if removing**

Edit `/home/cmndcntrl/code/rust-nexus/Cargo.toml`:
- If removing pelite/goblin, remove lines 57-59:
  ```toml
  # Windows PE/COFF parsing
  pelite = "0.10"
  goblin = "0.7"
  ```

**Substep 15.3: Update nexus-infra Cargo.toml if removing**

Edit `/home/cmndcntrl/code/rust-nexus/nexus-infra/Cargo.toml`:
- If removing, delete lines 57-59:
  ```toml
  # Windows PE/COFF parsing
  goblin = { workspace = true }
  pelite = { workspace = true }
  ```

**Validation:**
```bash
# Check workspace builds without these dependencies
cargo clean
cargo build --workspace 2>&1 | tee /tmp/step15-build.log

# Check for unused dependencies
cargo +nightly udeps --workspace 2>&1 | tee /tmp/step15-udeps.log || echo "Note: cargo-udeps not installed"
```

**Expected Outcome:**
- Decision documented on pelite/goblin (likely KEEP for potential future binary analysis needs)
- If removed, workspace builds cleanly
- If kept, documented for future evaluation

**Commit Point:** (Only if dependencies removed)
```bash
git add -A
git commit -m "Remove unused PE/COFF parsing dependencies

- Removed pelite and goblin dependencies (only used by bof_loader)
- Cleaned workspace and nexus-infra Cargo.toml
- Part of offensive code removal for gov-nexus transformation"
```

**OR** (If keeping):
Document in `/home/cmndcntrl/code/rust-nexus/docs/implementation/02-dependency-review.md`:
```markdown
## Dependencies Retained for Evaluation

### pelite and goblin (PE/COFF parsing)
- **Status:** Retained
- **Reason:** May be useful for binary analysis in compliance context
- **Action:** Evaluate in Phase 2 whether needed for legitimate use cases
```

---

#### Step 16: Full Workspace Rebuild and Test
**Objective:** Verify entire workspace builds and tests pass

**Substep 16.1: Clean rebuild**
```bash
cd /home/cmndcntrl/code/rust-nexus
cargo clean
cargo build --workspace --all-targets 2>&1 | tee /tmp/step16-build.log
```

**Substep 16.2: Run all tests**
```bash
cargo test --workspace --no-fail-fast 2>&1 | tee /tmp/step16-test.log
```

**Substep 16.3: Run clippy**
```bash
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee /tmp/step16-clippy.log
```

**Substep 16.4: Check formatting**
```bash
cargo fmt --all -- --check 2>&1 | tee /tmp/step16-fmt.log
```

**Substep 16.5: Compare with baseline**
```bash
# Compare build output
diff /tmp/build-status-before.log /tmp/step16-build.log > /tmp/build-diff.log || true
echo "Build differences documented in /tmp/build-diff.log"

# Compare test output
diff /tmp/test-status-before.log /tmp/step16-test.log > /tmp/test-diff.log || true
echo "Test differences documented in /tmp/test-diff.log"
```

**Validation Criteria:**
- ✓ All crates build successfully
- ✓ Tests pass (or clearly documented failures unrelated to removals)
- ✓ No clippy warnings
- ✓ Code formatted correctly
- ✓ No references to removed components in build output

**Expected Outcome:**
- Clean, working codebase
- All offensive code removed
- Ready for final review

**Document Results:**
Create `/home/cmndcntrl/code/rust-nexus/docs/implementation/01-removal-validation.md`:
```markdown
# Offensive Code Removal - Validation Report

**Date:** [Execution date]
**Validation:** Step 16 - Full Workspace Rebuild

## Build Status
- **Result:** [PASS/FAIL]
- **Details:** See /tmp/step16-build.log

## Test Status
- **Result:** [PASS/FAIL]
- **Tests Run:** [Count]
- **Tests Passed:** [Count]
- **Tests Failed:** [Count]
- **Details:** See /tmp/step16-test.log

## Code Quality
- **Clippy:** [PASS/FAIL]
- **Format:** [PASS/FAIL]

## Changes Summary
- Removed files: [List]
- Modified files: [List]
- Lines removed: [Approximate count]

## Remaining Issues
[Document any issues that need follow-up]
```

---

#### Step 17: Create Audit Trail Documentation
**Objective:** Document what was removed for compliance audit

**Action:**

Create `/home/cmndcntrl/code/rust-nexus/docs/implementation/01-removal-audit-trail.md`:

```markdown
# Offensive Code Removal - Audit Trail

**Transformation:** rust-nexus → gov-nexus
**Phase:** 1 - Offensive Code Removal
**Date:** [Execution date]
**Executed by:** [Name/Role]

## Removed Components

### 1. Shellcode Injection Infrastructure
**File:** `nexus-agent/src/fiber_execution.rs`
**Commit:** [Git commit hash]
**Date Removed:** [Date]
**Capabilities Removed:**
- Windows fiber-based shellcode execution
- Process hollowing via fiber API
- Early bird injection
- APC queue injection
- Direct fiber shellcode execution

**Justification:** These capabilities are purely offensive and have no legitimate use in a compliance monitoring platform.

**References Removed:**
- `nexus-agent/src/main.rs`: Module declaration
- `nexus-agent/src/execution.rs`: FiberExecutor usage
- `nexus-agent/src/agent.rs`: Capability registration

---

### 2. BOF/COFF Loader
**File:** `nexus-infra/src/bof_loader.rs`
**Commit:** [Git commit hash]
**Date Removed:** [Date]
**Capabilities Removed:**
- Beacon Object File (BOF) loading and execution
- COFF (Common Object File Format) dynamic loading
- In-memory code execution from object files
- BOF argument marshaling and execution

**Justification:** BOF loading is a technique associated with offensive security tools (Cobalt Strike). No legitimate compliance use case.

**References Removed:**
- `nexus-infra/src/lib.rs`: Module and re-exports
- `nexus-agent/src/execution.rs`: BOFLoader imports and usage

---

### 3. Domain Fronting and Traffic Obfuscation
**Crate:** `nexus-web-comms/`
**Commit:** [Git commit hash]
**Date Removed:** [Date]
**Capabilities Removed:**
- Domain fronting via CDN misrouting
- Traffic obfuscation and jitter
- HTTP fallback channels
- WebSocket fallback channels
- C2 channel hiding techniques

**Justification:** Domain fronting and traffic obfuscation are techniques used to evade network monitoring. Inappropriate for a compliance platform that should operate transparently.

**Impact:** Removed entire crate from workspace.

---

### 4. Keylogger Implementation
**File:** `nexus-agent/src/execution.rs`
**Lines Removed:** 440-619 (approximately 180 lines)
**Commit:** [Git commit hash]
**Date Removed:** [Date]
**Capabilities Removed:**
- Keylogger BOF loading and execution
- Keystroke capture and buffering
- Keylogger state management
- Keylogger data collection and exfiltration

**Justification:** Keylogging is an invasive surveillance technique with significant privacy implications. Not appropriate for compliance monitoring.

**References Removed:**
- `KEYLOGGER_BOF_DATA` embedded binary
- `KeyloggerState` management struct
- Keylogger task handlers (start/stop/status/flush)

---

### 5. BOF Source Files
**Directory:** `nexus-agent/bofs/`
**Commit:** [Git commit hash]
**Date Removed:** [Date]
**Contents Removed:**
- `keylogger/` - Keylogger BOF source code

**Justification:** Source code for offensive BOF tools. No longer needed after removing BOF execution capability.

---

### 6. Offensive Protocol Definitions
**File:** `nexus-infra/proto/nexus.proto`
**Commit:** [Git commit hash]
**Date Removed:** [Date]
**RPC Methods Removed:**
- `ExecuteShellcode(ShellcodeRequest) returns (ShellcodeResponse)`
- `ExecuteBOF(BOFRequest) returns (BOFResponse)`

**TaskType Enums Removed:**
- `TASK_TYPE_FIBER_SHELLCODE = 20`
- `TASK_TYPE_FIBER_HOLLOWING = 21`
- `TASK_TYPE_PROCESS_INJECTION = 22`
- `TASK_TYPE_DLL_INJECTION = 23`
- `TASK_TYPE_APC_INJECTION = 24`
- `TASK_TYPE_EARLY_BIRD_INJECTION = 25`
- `TASK_TYPE_BOF_EXECUTION = 26`
- `TASK_TYPE_COFF_LOADING = 27`
- `TASK_TYPE_KEYLOGGER_START = 44`
- `TASK_TYPE_KEYLOGGER_STOP = 45`
- `TASK_TYPE_CREDENTIAL_HARVESTING = 41`

**Message Types Removed:**
- `ShellcodeRequest`
- `ShellcodeResponse`
- `ShellcodeExecutionMethod` (enum)
- `BOFRequest`
- `BOFResponse`
- `BOFArgument`
- `BOFArgumentType` (enum)

**Justification:** Protocol definitions for offensive capabilities. Removed to prevent any future re-implementation.

---

## Code Statistics

**Total Files Removed:** 8
**Total Lines Removed:** ~21,500
**Total Commits:** 11-13 (depending on execution)

**Breakdown:**
- `fiber_execution.rs`: ~17,368 bytes
- `bof_loader.rs`: ~20,435 bytes
- `nexus-web-comms/`: ~26,000 bytes (entire crate)
- `execution.rs` keylogger code: ~180 lines
- `nexus.proto`: ~160 lines
- Documentation: ~8 files

---

## Verification

**Build Status:** [PASS/FAIL]
**Test Status:** [PASS/FAIL]
**Remaining Offensive Code:** None detected

**Verification Commands Run:**
```bash
# Searched for offensive patterns
grep -r "shellcode\|fiber.execution\|bof.loader\|keylogger" --include="*.rs"
grep -r "domain.fronting\|traffic.obfuscation" --include="*.rs"
grep -r "process.injection\|dll.injection" --include="*.rs"

# All searches returned no results
```

---

## Retained for Review

The following capabilities were RETAINED and marked for Phase 2 evaluation:

1. **TASK_TYPE_BROWSER_DATA_EXTRACTION**
   - **Reason:** May be legitimate for eDiscovery/data leak prevention
   - **Action Required:** Evaluate in compliance context

2. **TASK_TYPE_SCREEN_CAPTURE**
   - **Reason:** May be legitimate for user activity monitoring in compliance
   - **Action Required:** Evaluate privacy implications

3. **pelite/goblin dependencies**
   - **Reason:** May be useful for binary analysis in compliance
   - **Action Required:** Evaluate if needed, remove if not

---

## Attestation

I hereby attest that:
1. All identified offensive code has been removed from the rust-nexus codebase
2. All removals have been documented in this audit trail
3. The codebase builds successfully after removals
4. No functionality remains that is purely offensive in nature
5. All changes are committed to version control with clear commit messages

**Signed:** [Name]
**Date:** [Date]
**Role:** [Role]

---

## Appendix: Commit Hashes

| Step | Description | Commit Hash |
|------|-------------|-------------|
| 4 | Remove fiber_execution.rs | [hash] |
| 5 | Remove BOF directory | [hash] |
| 6 | Remove keylogger code | [hash] |
| 7 | Remove bof_loader.rs | [hash] |
| 8 | Remove offensive features | [hash] |
| 9 | Remove nexus-web-comms | [hash] |
| 11 | Remove offensive RPCs | [hash] |
| 12 | Remove offensive TaskTypes | [hash] |
| 13 | Remove offensive messages | [hash] |
| 14 | Update documentation | [hash] |

---

## Next Steps

Proceed to Phase 2: Capability Evaluation and Replacement
- Evaluate retained capabilities for compliance use
- Implement replacement legitimate functionality
- Add compliance-focused features
```

**Validation:**
Create the audit trail document and ensure it's tracked in git.

**Commit Point:**
```bash
git add -A
git commit -m "Add offensive code removal audit trail documentation

- Created comprehensive audit trail for compliance review
- Documented all removed components with justification
- Listed all commits related to offensive code removal
- Part of offensive code removal for gov-nexus transformation"
```

---

### Phase 7: Final Review and Merge (Steps 18-20)

#### Step 18: Peer Review Checklist
**Objective:** Ensure nothing was missed before merge

**Review Checklist:**

```markdown
# Offensive Code Removal - Review Checklist

## Files Completely Removed
- [ ] nexus-agent/src/fiber_execution.rs - DELETED
- [ ] nexus-infra/src/bof_loader.rs - DELETED
- [ ] nexus-web-comms/ (entire crate) - DELETED
- [ ] nexus-agent/bofs/ (entire directory) - DELETED
- [ ] docs/execution/bof-guide.md - DELETED
- [ ] docs/execution/keylogger-guide.md - DELETED

## Code Sections Removed
- [ ] Keylogger implementation (execution.rs lines 440-603) - REMOVED
- [ ] FiberExecutor field and usage - REMOVED
- [ ] Keylogger task handlers - REMOVED
- [ ] Fiber task handlers - REMOVED
- [ ] BOFLoader usage - REMOVED

## Protocol Definitions Removed
- [ ] ExecuteShellcode RPC - REMOVED
- [ ] ExecuteBOF RPC - REMOVED
- [ ] TASK_TYPE_FIBER_* enums - REMOVED
- [ ] TASK_TYPE_BOF_* enums - REMOVED
- [ ] TASK_TYPE_KEYLOGGER_* enums - REMOVED
- [ ] TASK_TYPE_CREDENTIAL_HARVESTING - REMOVED
- [ ] ShellcodeRequest/Response messages - REMOVED
- [ ] BOFRequest/Response messages - REMOVED
- [ ] ShellcodeExecutionMethod enum - REMOVED
- [ ] BOFArgument types - REMOVED

## References Cleaned
- [ ] No "fiber_execution" in codebase
- [ ] No "bof_loader" in codebase
- [ ] No "keylogger" in codebase
- [ ] No "nexus-web-comms" references
- [ ] No "domain fronting" references
- [ ] No "shellcode" in remaining code
- [ ] Capability strings cleaned in agent.rs

## Workspace Configuration
- [ ] nexus-web-comms removed from workspace members
- [ ] Offensive feature flags removed
- [ ] Cargo.toml dependencies reviewed
- [ ] No broken dependency references

## Build and Test
- [ ] Workspace builds cleanly: `cargo build --workspace`
- [ ] All tests pass: `cargo test --workspace`
- [ ] No clippy warnings: `cargo clippy --workspace`
- [ ] Code formatted: `cargo fmt --check`

## Documentation
- [ ] README.md updated
- [ ] DOCUMENTATION.md updated
- [ ] CLAUDE.md updated
- [ ] Examples updated
- [ ] Offensive guides removed
- [ ] Audit trail created

## Git Hygiene
- [ ] All changes committed with clear messages
- [ ] Commits reference "offensive code removal"
- [ ] Commits reference "gov-nexus transformation"
- [ ] No sensitive data in commit messages
- [ ] Backup branch exists

## Security Review
- [ ] No backdoors or hidden offensive code
- [ ] No obfuscated malicious code
- [ ] All removed code truly removed (not just commented)
- [ ] No embedded shellcode/BOF binaries remain
```

**Action:**
Have a second reviewer go through this checklist and verify each item.

---

#### Step 19: Final Validation Build
**Objective:** One last verification before merge

**Actions:**
```bash
cd /home/cmndcntrl/code/rust-nexus

# Clean everything
cargo clean
rm -rf target/

# Fresh build
cargo build --workspace --all-targets --verbose 2>&1 | tee /tmp/final-build.log

# Run tests
cargo test --workspace --verbose 2>&1 | tee /tmp/final-test.log

# Security audit (if cargo-audit installed)
cargo audit 2>&1 | tee /tmp/final-audit.log || echo "cargo-audit not installed"

# Check for offensive patterns one final time
echo "=== Final Offensive Code Scan ===" > /tmp/final-scan.log
grep -r "shellcode\|injection\|fiber.execution" --include="*.rs" . >> /tmp/final-scan.log || echo "✓ No shellcode/injection references" >> /tmp/final-scan.log
grep -r "bof.loader\|BOFLoader\|LoadedBof" --include="*.rs" . >> /tmp/final-scan.log || echo "✓ No BOF loader references" >> /tmp/final-scan.log
grep -r "keylogger\|keystroke" --include="*.rs" . >> /tmp/final-scan.log || echo "✓ No keylogger references" >> /tmp/final-scan.log
grep -r "domain.fronting\|DomainFronting" --include="*.rs" . >> /tmp/final-scan.log || echo "✓ No domain fronting" >> /tmp/final-scan.log

cat /tmp/final-scan.log
```

**Validation Criteria:**
- Build: PASS
- Tests: PASS (or documented exceptions)
- No offensive patterns found
- No compilation errors
- No test failures related to removals

**Expected Outcome:**
- Completely clean codebase
- Ready for merge to main
- Confidence in removal completeness

---

#### Step 20: Merge to Main
**Objective:** Integrate changes into main branch

**Substep 20.1: Update feature branch**
```bash
cd /home/cmndcntrl/code/rust-nexus
git checkout main
git pull origin main
git checkout feature/remove-offensive-code
git merge main  # Resolve any conflicts
```

**Substep 20.2: Final commit if needed**
```bash
# If any final cleanup needed
git add -A
git commit -m "Final cleanup for offensive code removal

- Verified all offensive code removed
- Passed all build and test validation
- Ready for merge to main
- Completes Phase 1 of gov-nexus transformation"
```

**Substep 20.3: Push feature branch**
```bash
git push origin feature/remove-offensive-code
```

**Substep 20.4: Create merge commit or PR**

**Option A: Direct merge (if process allows):**
```bash
git checkout main
git merge --no-ff feature/remove-offensive-code -m "Merge: Remove all offensive code for gov-nexus transformation

This merge completes Phase 1 of the rust-nexus to gov-nexus transformation.

Summary of changes:
- Removed shellcode injection infrastructure (fiber_execution.rs)
- Removed BOF/COFF loader (bof_loader.rs)
- Removed domain fronting crate (nexus-web-comms)
- Removed keylogger implementation
- Removed offensive protocol definitions
- Updated documentation to reflect compliance focus

Total: ~21,500 lines of offensive code removed
Commits: 11-13 atomic commits
Build status: PASS
Test status: PASS

See docs/implementation/01-offensive-code-removal.md for details.
See docs/implementation/01-removal-audit-trail.md for audit trail."

git push origin main
```

**Option B: Create Pull Request (recommended):**
```bash
# Use GitHub CLI or web interface
gh pr create \
  --title "Phase 1: Remove all offensive code for gov-nexus transformation" \
  --body "$(cat <<'EOF'
# Offensive Code Removal - Phase 1 Complete

This PR removes all purely offensive components from rust-nexus as part of the transformation to gov-nexus compliance platform.

## Changes Summary

### Removed Files (8 total)
- ✅ `nexus-agent/src/fiber_execution.rs` - Shellcode injection
- ✅ `nexus-infra/src/bof_loader.rs` - BOF/COFF loader
- ✅ `nexus-web-comms/` - Domain fronting crate (entire)
- ✅ `nexus-agent/bofs/` - BOF source files
- ✅ `docs/execution/bof-guide.md`
- ✅ `docs/execution/keylogger-guide.md`

### Modified Files
- `nexus-agent/src/main.rs` - Removed fiber_execution module
- `nexus-agent/src/execution.rs` - Removed keylogger code (~180 lines)
- `nexus-agent/src/agent.rs` - Removed offensive capabilities
- `nexus-infra/src/lib.rs` - Removed bof_loader module
- `nexus-infra/proto/nexus.proto` - Removed offensive RPC methods and message types
- `Cargo.toml` - Removed nexus-web-comms from workspace
- `nexus-agent/Cargo.toml` - Removed offensive feature flags
- `README.md`, `DOCUMENTATION.md`, `CLAUDE.md` - Updated for compliance focus

## Offensive Capabilities Removed

### Shellcode Injection
- Fiber-based shellcode execution
- Process hollowing
- Early bird injection
- APC injection
- DLL injection

### BOF/COFF Loading
- Beacon Object File execution
- COFF dynamic loading
- In-memory code execution

### C2 Evasion
- Domain fronting
- Traffic obfuscation
- HTTP/WebSocket fallback channels

### Surveillance
- Keylogger implementation
- Credential harvesting task types

## Protocol Changes

Removed RPC methods:
- `ExecuteShellcode`
- `ExecuteBOF`

Removed TaskType enums:
- All FIBER_* types (20-25)
- All BOF/COFF types (26-27)
- KEYLOGGER types (44-45)
- CREDENTIAL_HARVESTING (41)

Removed message types:
- ShellcodeRequest/Response
- BOFRequest/Response
- ShellcodeExecutionMethod
- BOFArgument types

## Validation

- ✅ Build: PASS (`cargo build --workspace`)
- ✅ Tests: PASS (`cargo test --workspace`)
- ✅ Clippy: PASS (`cargo clippy --workspace`)
- ✅ Format: PASS (`cargo fmt --check`)
- ✅ No offensive patterns found in final scan
- ✅ ~21,500 lines of offensive code removed
- ✅ Audit trail documented

## Documentation

- 📄 Implementation plan: `docs/implementation/01-offensive-code-removal.md`
- 📄 Audit trail: `docs/implementation/01-removal-audit-trail.md`
- 📄 Validation report: `docs/implementation/01-removal-validation.md`

## Review Checklist

- [ ] Code review: Verify all offensive code removed
- [ ] Security review: No hidden offensive functionality
- [ ] Documentation review: Appropriate for compliance platform
- [ ] Build verification: Clean workspace build
- [ ] Test verification: All tests pass

## Next Steps

After merge:
1. Proceed to Phase 2: Capability Evaluation
2. Evaluate retained capabilities (browser data, screen capture)
3. Design compliance-focused replacements
4. Implement gov-nexus specific features

---

**Transformation:** rust-nexus → gov-nexus
**Phase:** 1 of 3
**Status:** Ready for review
EOF
)" \
  --base main \
  --head feature/remove-offensive-code \
  --reviewer [reviewer-github-username]
```

**Validation:**
- PR created successfully
- CI/CD pipeline runs (if configured)
- Reviewers assigned
- All checks pass

**Expected Outcome:**
- Changes merged to main branch
- Offensive code removal complete
- Ready for Phase 2

---

## Validation Procedures

### After Each Step
1. **Verify file state:** Use `git status` and `ls` to confirm expected changes
2. **Check compilation:** Run `cargo check -p <affected-crate>`
3. **Search for references:** Use `grep` to verify no dangling references
4. **Review diff:** Use `git diff` to confirm only intended changes
5. **Commit atomically:** Create focused commit with clear message

### After Each Phase
1. **Build workspace:** `cargo build --workspace`
2. **Run tests:** `cargo test --workspace`
3. **Check references:** Search for removed component names
4. **Review commits:** Ensure clear commit history
5. **Update documentation:** Keep this plan synchronized with actual execution

### Final Validation (Step 16, 19)
1. **Clean rebuild:** `cargo clean && cargo build --workspace --all-targets`
2. **Full test suite:** `cargo test --workspace --no-fail-fast`
3. **Code quality:** `cargo clippy --workspace -- -D warnings`
4. **Format check:** `cargo fmt --all -- --check`
5. **Security scan:** `cargo audit` (if available)
6. **Offensive pattern scan:** grep for all offensive patterns
7. **Diff with baseline:** Compare build/test output with Step 2 baseline

---

## Rollback Procedures

### If Issues Found During Step
1. **Stop immediately:** Don't proceed to next step
2. **Review changes:** `git diff HEAD`
3. **Identify issue:** Determine if fixable or requires rollback
4. **Rollback if needed:** `git reset --hard HEAD` (loses uncommitted changes)
5. **Restore from commit:** `git reset --hard <last-good-commit>`
6. **Re-attempt step:** Fix issue and retry with updated procedure

### If Issues Found After Merge
1. **Create hotfix branch:** `git checkout -b hotfix/offensive-code-removal-fix`
2. **Fix issue:** Make minimal corrective changes
3. **Test thoroughly:** Ensure fix resolves issue
4. **Merge or revert:**
   - If fixable: Merge hotfix to main
   - If major issues: `git revert <merge-commit>` to restore previous state
5. **Restore from backup:** Backup branch created in Step 1 can be used

### Nuclear Option (Last Resort)
```bash
# If everything goes wrong, restore from backup branch
git checkout main
git reset --hard backup/pre-offensive-removal
git push --force origin main  # DANGEROUS - coordinate with team

# Then re-attempt with updated plan
```

---

## Audit Trail

### Execution Log
During execution, maintain a log in `/home/cmndcntrl/code/rust-nexus/docs/implementation/01-execution-log.md`:

```markdown
# Offensive Code Removal - Execution Log

## Step 1: Create Backup Branch
- **Date:** [Date]
- **Time:** [Time]
- **Executed by:** [Name]
- **Status:** [SUCCESS/FAILED]
- **Notes:** [Any observations]
- **Commit:** [Git hash if applicable]

## Step 2: Document Current Build State
- **Date:** [Date]
- **Time:** [Time]
- **Executed by:** [Name]
- **Status:** [SUCCESS/FAILED]
- **Build Status:** [Output summary]
- **Test Status:** [Output summary]
- **Notes:** [Baseline observations]

[Continue for each step...]

## Issues Encountered

### Issue 1: [Description]
- **Step:** [Step number]
- **Date:** [Date]
- **Description:** [Detailed description]
- **Resolution:** [How it was resolved]
- **Impact:** [Impact on timeline/plan]

[Document all issues...]

## Deviations from Plan

### Deviation 1: [Description]
- **Step:** [Step number]
- **Reason:** [Why deviation was necessary]
- **Change:** [What was done differently]
- **Approval:** [Who approved the deviation]

[Document all deviations...]
```

### Commit Message Template
Each commit should follow this format:
```
[Scope]: Brief description (50 chars max)

Detailed explanation of what was removed and why.

Changes:
- Bullet list of specific changes
- Be explicit about files/functions removed

Justification:
- Why this code was offensive
- Why it has no place in gov-nexus

Part of offensive code removal for gov-nexus transformation.
Phase 1, Step [N]

Related: [Issue tracker link if applicable]
```

---

## Success Criteria

### Phase 1 Complete When:
- [ ] All 6 offensive components completely removed
- [ ] Workspace builds successfully
- [ ] Tests pass (or failures documented as unrelated)
- [ ] No grep hits for offensive patterns
- [ ] Documentation updated
- [ ] Audit trail complete
- [ ] Peer review passed
- [ ] Merged to main branch

### Quality Metrics:
- **Code Removed:** ~21,500 lines
- **Files Deleted:** 8+ files
- **Commits:** 11-13 atomic commits
- **Build Time:** No significant increase
- **Test Coverage:** No decrease
- **Documentation:** Complete and accurate

---

## Appendix A: Grep Patterns for Verification

Use these patterns to verify complete removal:

```bash
# Shellcode and injection
grep -r "shellcode\|Shellcode" --include="*.rs" .
grep -r "fiber.execution\|FiberExecutor" --include="*.rs" .
grep -r "process.injection\|ProcessInjection" --include="*.rs" .
grep -r "dll.injection\|DllInjection" --include="*.rs" .
grep -r "apc.injection\|ApcInjection" --include="*.rs" .
grep -r "early.bird\|EarlyBird" --include="*.rs" .
grep -r "fiber.hollowing\|FiberHollowing" --include="*.rs" .

# BOF/COFF
grep -r "bof.loader\|BOFLoader\|BofLoader" --include="*.rs" .
grep -r "LoadedBof\|loaded.bof" --include="*.rs" .
grep -r "BofArgument\|BOFArgument" --include="*.rs" .
grep -r "COFF\|coff" --include="*.rs" .

# Keylogger
grep -r "keylogger\|Keylogger" --include="*.rs" .
grep -r "keystroke\|Keystroke" --include="*.rs" .
grep -r "KeyloggerState" --include="*.rs" .
grep -r "KEYLOGGER_BOF_DATA" --include="*.rs" .

# Domain fronting
grep -r "domain.fronting\|DomainFronting" --include="*.rs" .
grep -r "nexus.web.comms\|nexus_web_comms" --include="*.rs" --include="*.toml" .
grep -r "traffic.obfuscation\|TrafficObfuscation" --include="*.rs" .

# Credential harvesting
grep -r "credential.harvest\|CredentialHarvest" --include="*.rs" .
grep -r "CREDENTIAL_HARVESTING" --include="*.proto" .
```

All should return no results or only comments/documentation noting removal.

---

## Appendix B: File Size Reference

For validation during removal:

| File/Directory | Size | Lines |
|---------------|------|-------|
| fiber_execution.rs | 17,368 bytes | ~450 |
| bof_loader.rs | 20,435 bytes | ~550 |
| nexus-web-comms/ | ~26,000 bytes | ~700 |
| execution.rs keylogger | ~5,000 bytes | ~180 |
| nexus.proto offensive | ~4,500 bytes | ~160 |
| **Total** | **~73,303 bytes** | **~2,040** |

---

## Appendix C: Dependencies Graph

```
┌─────────────────┐
│  nexus-agent    │
│  ┌──────────┐   │
│  │fiber_exec│◄──┼─── REMOVE (fiber_execution.rs)
│  └──────────┘   │
│  ┌──────────┐   │
│  │execution │◄──┼─── MODIFY (remove keylogger, fiber refs)
│  └──────────┘   │
│  ┌──────────┐   │
│  │  agent   │◄──┼─── MODIFY (remove capabilities)
│  └──────────┘   │
└────────┬────────┘
         │ depends on
         ▼
┌─────────────────┐
│  nexus-infra    │
│  ┌──────────┐   │
│  │bof_loader│◄──┼─── REMOVE (bof_loader.rs)
│  └──────────┘   │
│  ┌──────────┐   │
│  │  proto   │◄──┼─── MODIFY (remove offensive messages)
│  └──────────┘   │
│  ┌──────────┐   │
│  │   lib    │◄──┼─── MODIFY (remove bof_loader refs)
│  └──────────┘   │
└─────────────────┘

┌─────────────────┐
│nexus-web-comms  │◄─── REMOVE ENTIRE CRATE
│ (domain fronting)│
└─────────────────┘

┌─────────────────┐
│ nexus-agent/bofs│◄─── REMOVE ENTIRE DIRECTORY
│   (BOF sources) │
└─────────────────┘
```

---

## Appendix D: Timeline Estimate

Assuming careful execution with validation at each step:

| Phase | Steps | Estimated Time | Notes |
|-------|-------|----------------|-------|
| Phase 1 | 1-3 | 30 minutes | Pre-flight checks |
| Phase 2 | 4-6 | 2-3 hours | File removals, code editing |
| Phase 3 | 7-8 | 1 hour | BOF infrastructure |
| Phase 4 | 9-10 | 45 minutes | Crate removal |
| Phase 5 | 11-13 | 1-2 hours | Proto definitions |
| Phase 6 | 14-17 | 2-3 hours | Documentation, validation |
| Phase 7 | 18-20 | 1 hour | Final review, merge |
| **Total** | **20** | **8-11 hours** | Full Baby Steps execution |

**Accelerated:** If very familiar with codebase: 4-6 hours
**Conservative:** If careful review needed: 12-16 hours
**With issues:** Allow 2x time for unexpected problems

---

## Appendix E: Related Documentation

After completion, update these documents:

1. **Architecture Document:** `/home/cmndcntrl/code/rust-nexus/docs/ARCHITECTURE.md`
   - Remove offensive component diagrams
   - Update for gov-nexus focus

2. **API Documentation:** `/home/cmndcntrl/code/rust-nexus/docs/api/README.md`
   - Remove offensive RPC documentation
   - Update task type listings

3. **Development Guide:** `/home/cmndcntrl/code/rust-nexus/docs/development/DEVELOPMENT.md`
   - Remove BOF development instructions
   - Update build instructions

4. **Deployment Guide:** `/home/cmndcntrl/code/rust-nexus/docs/operations/DEPLOYMENT.md`
   - Remove offensive capability deployment examples
   - Update for compliance deployment scenarios

5. **Security Documentation:** `/home/cmndcntrl/code/rust-nexus/docs/SECURITY.md`
   - Update threat model for compliance platform
   - Remove offensive technique descriptions

---

## Document Maintenance

This implementation plan is a living document during Phase 1 execution.

**Update this document when:**
- Deviations from plan are necessary
- Additional dependencies discovered
- Issues encountered requiring procedural changes
- Validation procedures prove insufficient
- Timeline estimates prove inaccurate

**Version History:**
- v1.0 (2025-12-21): Initial detailed implementation plan created

---

**End of Implementation Plan**

**Status:** Ready for execution
**Approval Required:** [Yes/No]
**Approver:** [Name/Role]
**Approved Date:** [Date]
