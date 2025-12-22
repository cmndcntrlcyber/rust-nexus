# Agent Transformation Implementation Plan: nexus-agent to gov-agent

## Overview
Transform offensive nexus-agent into defensive gov-agent using Baby Steps methodology. Each step represents the smallest possible meaningful change.

## Critical Constraints
- NEVER execute or test malicious code
- Only analyze and document existing offensive capabilities
- Focus on structural transformations, not functional execution
- Create defensive variants alongside offensive code initially
- Remove offensive code only after defensive replacements are tested

---

## Phase 1: Project Structure Setup (Steps 1-8)

### Step 1: Create gov-agent Directory Structure
**Action**: Create new crate directory `/home/cmndcntrl/code/rust-nexus/gov-agent`
**Validation**: Directory exists and is empty
**Process**: Single mkdir operation

### Step 2: Initialize gov-agent Cargo.toml
**Action**: Create minimal Cargo.toml with crate metadata
**Validation**: `cargo check` passes in gov-agent directory
**Process**: Copy from nexus-agent, update name/description

### Step 3: Create gov-agent Source Directory
**Action**: Create `/home/cmndcntrl/code/rust-nexus/gov-agent/src` directory
**Validation**: Directory exists
**Process**: Single mkdir operation

### Step 4: Create Placeholder main.rs
**Action**: Create minimal main.rs with hello world
**Validation**: `cargo build` succeeds
**Process**: Write basic main function

### Step 5: Update Workspace Cargo.toml
**Action**: Add gov-agent to workspace members list
**Validation**: `cargo check --workspace` passes
**Process**: Add single line to workspace members array

### Step 6: Create Module Structure Documentation
**Action**: Create `/home/cmndcntrl/code/rust-nexus/gov-agent/MODULE_MAP.md`
**Validation**: File contains mapping table
**Process**: Document old->new module names

### Step 7: Create Tests Directory
**Action**: Create `/home/cmndcntrl/code/rust-nexus/gov-agent/tests` directory
**Validation**: Directory exists
**Process**: Single mkdir operation

### Step 8: Verify Build Chain
**Action**: Run `cargo build -p gov-agent`
**Validation**: Build completes without errors
**Process**: Single cargo command

---

## Phase 2: asset.rs - System Info to Asset Inventory (Steps 9-16)

### Step 9: Create asset.rs File
**Action**: Create empty `/home/cmndcntrl/code/rust-nexus/gov-agent/src/asset.rs`
**Validation**: File exists
**Process**: Touch file, add module declaration

### Step 10: Copy SystemInfo Struct to AssetInventory
**Action**: Copy SystemInfo struct from system.rs, rename to AssetInventory
**Validation**: Struct compiles
**Process**: Copy-paste, rename struct name only

### Step 11: Add Basic Asset Fields
**Action**: Add `compliance_tags: Vec<String>` to AssetInventory
**Validation**: Struct compiles with new field
**Process**: Add single field with default initialization

### Step 12: Add Last Scan Timestamp
**Action**: Add `last_scan: chrono::DateTime<Utc>` field
**Validation**: Struct compiles, chrono dependency added
**Process**: Add field and update Cargo.toml

### Step 13: Add Installed Software Field
**Action**: Add `installed_software: Vec<SoftwarePackage>` field
**Validation**: Compiles with placeholder SoftwarePackage struct
**Process**: Add field and stub struct

### Step 14: Add Security Tools Field
**Action**: Add `security_tools: Vec<SecurityTool>` field
**Validation**: Compiles with placeholder SecurityTool struct
**Process**: Add field and stub struct

### Step 15: Add Configuration Baseline Field
**Action**: Add `configuration_baseline: HashMap<String, String>` field
**Validation**: Struct compiles
**Process**: Add single field

### Step 16: Implement AssetInventory::collect() Method
**Action**: Copy SystemInfo::collect(), update return type
**Validation**: Method compiles and returns AssetInventory
**Process**: Copy method, update type signature

---

## Phase 3: security_validation.rs - Evasion to Security Validation (Steps 17-26)

### Step 17: Create security_validation.rs File
**Action**: Create `/home/cmndcntrl/code/rust-nexus/gov-agent/src/security_validation.rs`
**Validation**: File exists
**Process**: Touch file, add module declaration

### Step 18: Create SecurityValidator Struct
**Action**: Create empty SecurityValidator struct
**Validation**: Struct compiles
**Process**: Add basic struct definition

### Step 19: Create ControlAssessment Struct
**Action**: Define ControlAssessment with status, control_id, findings fields
**Validation**: Struct compiles
**Process**: Add struct definition

### Step 20: Create validate_endpoint_protection() Method Stub
**Action**: Add method signature returning ControlAssessment
**Validation**: Method compiles with TODO body
**Process**: Add method stub

### Step 21: Copy VM Detection Logic
**Action**: Copy check_vm_artifacts() to validate_endpoint_protection()
**Validation**: Logic compiles but returns stub ControlAssessment
**Process**: Copy function body, rename

### Step 22: Invert Detection Logic - VM Check
**Action**: Change return true to return pass ControlAssessment
**Validation**: Logic inversion compiles
**Process**: Flip boolean return to ControlAssessment::pass()

### Step 23: Add EDR Process List
**Action**: Create EDR_PROCESSES constant array with known EDR names
**Validation**: Constant compiles
**Process**: Add const array

### Step 24: Copy Process Detection Logic
**Action**: Copy check_debugging_tools() process enumeration
**Validation**: Compiles with renamed function
**Process**: Copy function, rename to check_edr_processes()

### Step 25: Invert EDR Detection Logic
**Action**: Return pass when EDR found, fail when not found
**Validation**: Logic inversion correct
**Process**: Flip if conditions

### Step 26: Add Framework Mapping
**Action**: Add framework_mappings: Vec<String> to ControlAssessment
**Validation**: Struct compiles with NIST/CIS mappings
**Process**: Add field with sample values

---

## Phase 4: persistence_audit.rs - Persistence to Audit (Steps 27-34)

### Step 27: Create persistence_audit.rs File
**Action**: Create `/home/cmndcntrl/code/rust-nexus/gov-agent/src/persistence_audit.rs`
**Validation**: File exists
**Process**: Touch file, add module declaration

### Step 28: Create PersistenceAuditor Struct
**Action**: Define PersistenceAuditor struct
**Validation**: Struct compiles
**Process**: Add basic struct

### Step 29: Create PersistenceFinding Struct
**Action**: Define PersistenceFinding with location, mechanism, risk_level fields
**Validation**: Struct compiles
**Process**: Add struct definition

### Step 30: Create RiskLevel Enum
**Action**: Define RiskLevel enum (Low, Medium, High, Critical)
**Validation**: Enum compiles with Debug, Clone derives
**Process**: Add enum definition

### Step 31: Create audit_registry_persistence() Method Stub
**Action**: Add method signature returning Vec<PersistenceFinding>
**Validation**: Method compiles with TODO
**Process**: Add method stub

### Step 32: Copy Registry Read Logic
**Action**: Copy registry query logic from install_registry_persistence()
**Validation**: Compiles but only reads, doesn't write
**Process**: Copy read portions only

### Step 33: Transform to Audit Logic
**Action**: Change from reg add to reg query for common persistence keys
**Validation**: Reads autorun keys instead of writing
**Process**: Change operation, add known persistence locations

### Step 34: Create audit_systemd_units() Method
**Action**: Add method to enumerate and assess systemd units
**Validation**: Method compiles with stub implementation
**Process**: Add method reading /etc/systemd/system/

---

## Phase 5: compliance_executor.rs - Execution to Compliance (Steps 35-44)

### Step 35: Create compliance_executor.rs File
**Action**: Create `/home/cmndcntrl/code/rust-nexus/gov-agent/src/compliance_executor.rs`
**Validation**: File exists
**Process**: Touch file, add module declaration

### Step 36: Create ComplianceExecutor Struct
**Action**: Define ComplianceExecutor struct
**Validation**: Struct compiles
**Process**: Add basic struct

### Step 37: Create CheckType Enum
**Action**: Define CheckType with variants: RegistryQuery, FilePermissions, ServiceStatus, ScapOval, CisBenchmark
**Validation**: Enum compiles
**Process**: Add enum definition

### Step 38: Create ComplianceCheck Struct
**Action**: Define ComplianceCheck with check_id, check_type, expected_value fields
**Validation**: Struct compiles
**Process**: Add struct definition

### Step 39: Copy Safe Execution Methods
**Action**: Copy shell, registry_query, file read methods from TaskExecutor
**Validation**: Methods compile in new context
**Process**: Copy read-only operations only

### Step 40: Remove Offensive Methods
**Action**: Ensure NO fiber_*, keylogger_*, injection methods exist
**Validation**: grep confirms no offensive code
**Process**: Verification step only

### Step 41: Create execute_registry_check() Method
**Action**: Add method wrapping query_registry for compliance
**Validation**: Method compiles and returns compliance result
**Process**: Add wrapper method

### Step 42: Create execute_file_permissions_check() Method
**Action**: Add method checking file permissions against baseline
**Validation**: Method compiles
**Process**: Add method using std::fs::metadata

### Step 43: Create execute_service_status_check() Method
**Action**: Add method checking if required services are running
**Validation**: Method compiles
**Process**: Add method using platform-specific service queries

### Step 44: Add execute_oval_check() Stub
**Action**: Create stub method for SCAP OVAL execution
**Validation**: Method exists with TODO
**Process**: Add method signature only

---

## Phase 6: Integration and Testing (Steps 45-50)

### Step 45: Create gov-agent main.rs
**Action**: Create main.rs importing all new modules
**Validation**: All modules compile together
**Process**: Add mod declarations

### Step 46: Update mod.rs Declarations
**Action**: Add pub mod statements for all four modules
**Validation**: `cargo check` passes
**Process**: Add module visibility

### Step 47: Create Integration Test
**Action**: Create `/home/cmndcntrl/code/rust-nexus/gov-agent/tests/integration_test.rs`
**Validation**: Test compiles
**Process**: Add basic smoke test

### Step 48: Verify No Offensive Code
**Action**: Run grep for offensive patterns across gov-agent
**Validation**: No matches for "keylog", "inject", "fiber", "shellcode"
**Process**: Run grep commands

### Step 49: Build gov-agent
**Action**: Run `cargo build -p gov-agent --release`
**Validation**: Clean build completes
**Process**: Single cargo command

### Step 50: Document Transformation Complete
**Action**: Update this file with completion status
**Validation**: Document reflects finished state
**Process**: Add completion notes

---

## Success Criteria

Each step must meet these criteria before proceeding:

1. **Compiles**: Code must compile without errors
2. **No Offensive Functionality**: No execution of malicious code
3. **Documented**: Change logged in this document
4. **Version Controlled**: Change committed to git
5. **Reversible**: Can undo change if needed

## Key Transformations Summary

| Offensive Module | Defensive Module | Key Change |
|-----------------|------------------|------------|
| system.rs | asset.rs | Add inventory fields for compliance |
| evasion.rs | security_validation.rs | Flip detection to validation |
| persistence.rs | persistence_audit.rs | Read-only audit vs. write-install |
| execution.rs | compliance_executor.rs | Remove offensive, keep queries |

## File Operations Summary

**Files to Create**: 4 new modules + main.rs + tests
**Files to Modify**: Cargo.toml (workspace and crate)
**Files to Remove**: None (keep nexus-agent intact)

## Dependencies to Add

- chrono (for timestamps)
- serde_json (for compliance output)
- No offensive dependencies (no windows-rs advanced features)

---

**Process Reminder**: The process is the product. Each step documents HOW we transform offensive to defensive capabilities.
