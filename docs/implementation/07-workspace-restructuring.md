# Workspace Restructuring Implementation Plan
## Renaming Crates: nexus-* to gov-*

**Document Version:** 1.0
**Created:** 2025-12-21
**Status:** Planning Phase
**Methodology:** Baby Steps™ - The Process is the Product

---

## Executive Summary

This document outlines the atomic, step-by-step plan for restructuring the Rust workspace by renaming all `nexus-*` crates to `gov-*` naming convention, deleting obsolete crates, and creating new governance-focused crates. Each step is designed to maintain a working build state and allow for incremental validation.

---

## Current Workspace State

```
nexus-agent          → gov-agent
nexus-common         → gov-common
nexus-infra          → gov-infra
nexus-webui          → gov-dashboard
nexus-recon          → gov-discovery
nexus-hybrid-exec    → gov-collectors
nexus-web-comms      → [DELETE]
```

### Dependencies Discovered
- **nexus-agent** depends on: `nexus-common`, `nexus-infra`
- **nexus-infra** has proto file: `proto/nexus.proto` (package: `nexus.v1`)
- **nexus-agent** has build.rs with hardcoded references to `nexus_keylogger`
- **nexus-infra** has build.rs that references `proto/nexus.proto`

### Files with "nexus-" References
```
/home/cmndcntrl/code/rust-nexus/Cargo.toml
/home/cmndcntrl/code/rust-nexus/config/agent-windows.toml
/home/cmndcntrl/code/rust-nexus/config/agent-linux.toml
/home/cmndcntrl/code/rust-nexus/nexus.toml
/home/cmndcntrl/code/rust-nexus/nexus-**/Cargo.toml (7 files)
```

---

## Target Workspace Structure

### Renamed Crates (Phase 1)
```
gov-common           # Core utilities, encryption, types
gov-infra            # gRPC services, proto definitions
gov-agent            # Agent implementation
gov-dashboard        # Web UI (formerly nexus-webui)
gov-discovery        # Reconnaissance (formerly nexus-recon)
gov-collectors       # Hybrid execution (formerly nexus-hybrid-exec)
```

### New Crates (Phase 2)
```
gov-compliance-engine # Compliance rule engine
gov-evidence          # Evidence collection and chain-of-custody
gov-policy            # Policy management and enforcement
gov-reporting         # Report generation
gov-api               # REST API layer
gov-tenancy           # Multi-tenancy support
gov-integrations      # Third-party integrations
```

---

## Baby Steps™ Implementation Plan

### Critical Principles
1. **One atomic change at a time** - Never combine multiple renames in a single step
2. **Build validation after each step** - Every step must result in a working build
3. **Commit after validation** - Each successful step gets its own commit
4. **Document as you go** - Update this file with actual outcomes
5. **Rollback plan** - Each step can be reverted independently

---

## Phase 1: Rename Existing Crates

### Step 1: Prepare Foundation - Delete Obsolete Crate
**Objective:** Remove nexus-web-comms to simplify workspace

#### Step 1.1: Remove from workspace members
- **Action:** Edit `/home/cmndcntrl/code/rust-nexus/Cargo.toml`
- **Change:** Remove `"nexus-web-comms"` from `workspace.members` array
- **Validation:** `cargo metadata --no-deps | grep nexus-web-comms` (should be empty)
- **Commit:** "Remove nexus-web-comms from workspace members"

#### Step 1.2: Delete directory
- **Action:** `rm -rf /home/cmndcntrl/code/rust-nexus/nexus-web-comms`
- **Validation:** Directory no longer exists
- **Build Check:** `cargo check --workspace`
- **Commit:** "Delete nexus-web-comms crate"

---

### Step 2: Rename Foundation Crate (nexus-common → gov-common)
**Objective:** Rename the foundational crate with no internal dependencies

#### Step 2.1: Rename directory
- **Action:** `mv nexus-common gov-common`
- **Location:** `/home/cmndcntrl/code/rust-nexus/`
- **Validation:** Directory exists at new location
- **Status:** Expect broken build state

#### Step 2.2: Update package name in Cargo.toml
- **File:** `/home/cmndcntrl/code/rust-nexus/gov-common/Cargo.toml`
- **Change:** Line 2: `name = "nexus-common"` → `name = "gov-common"`
- **Validation:** Grep confirms change
- **Status:** Still broken (workspace not updated)

#### Step 2.3: Update workspace members
- **File:** `/home/cmndcntrl/code/rust-nexus/Cargo.toml`
- **Change:** Line 5: `"nexus-common"` → `"gov-common"`
- **Validation:** `cargo metadata` shows gov-common
- **Status:** Still broken (dependents not updated)

#### Step 2.4: Update dependent crate (nexus-agent)
- **File:** `/home/cmndcntrl/code/rust-nexus/nexus-agent/Cargo.toml`
- **Change:** Line 7: `nexus-common = { path = "../nexus-common" }` → `gov-common = { path = "../gov-common" }`
- **Validation:** Grep confirms change

#### Step 2.5: Update import statements in nexus-agent
- **Action:** Find and replace across all .rs files in nexus-agent
- **Pattern:** `use nexus_common::` → `use gov_common::`
- **Pattern:** `extern crate nexus_common` → `extern crate gov_common`
- **Tool:** Use grep to find, then edit to replace
- **Validation:** `grep -r "nexus_common" nexus-agent/src` (should be empty)

#### Step 2.6: Update dependent crate (nexus-infra)
- **File:** `/home/cmndcntrl/code/rust-nexus/nexus-infra/Cargo.toml`
- **Action:** Search for nexus-common dependencies
- **Change:** If found, update to gov-common
- **Validation:** Grep confirms change

#### Step 2.7: Update import statements in nexus-infra
- **Action:** Find and replace across all .rs files in nexus-infra
- **Pattern:** `use nexus_common::` → `use gov_common::`
- **Validation:** `grep -r "nexus_common" nexus-infra/src` (should be empty)

#### Step 2.8: Update remaining crates
- **Crates to check:** nexus-webui, nexus-recon, nexus-hybrid-exec
- **Action for each:**
  1. Check Cargo.toml for nexus-common dependency
  2. If found, update to gov-common and update path
  3. Update all import statements in src/
  4. Validate with grep

#### Step 2.9: Build validation
- **Action:** `cargo check --workspace`
- **Expected:** Success
- **On failure:** Review errors, fix, repeat validation
- **Commit:** "Rename nexus-common to gov-common"

---

### Step 3: Rename Infrastructure Crate (nexus-infra → gov-infra)
**Objective:** Rename the infrastructure crate with proto definitions

#### Step 3.1: Rename directory
- **Action:** `mv nexus-infra gov-infra`
- **Location:** `/home/cmndcntrl/code/rust-nexus/`
- **Validation:** Directory exists at new location

#### Step 3.2: Update package name in Cargo.toml
- **File:** `/home/cmndcntrl/code/rust-nexus/gov-infra/Cargo.toml`
- **Change:** `name = "nexus-infra"` → `name = "gov-infra"`
- **Validation:** Grep confirms change

#### Step 3.3: Update proto package name
- **File:** `/home/cmndcntrl/code/rust-nexus/gov-infra/proto/nexus.proto`
- **Change:** Line 3: `package nexus.v1;` → `package gov.v1;`
- **Validation:** Grep confirms change

#### Step 3.4: Rename proto file
- **Action:** `mv gov-infra/proto/nexus.proto gov-infra/proto/gov.proto`
- **Validation:** File exists at new location

#### Step 3.5: Update build.rs to reference new proto file
- **File:** `/home/cmndcntrl/code/rust-nexus/gov-infra/build.rs`
- **Change:** Line 5: `let proto_file = "proto/nexus.proto";` → `let proto_file = "proto/gov.proto";`
- **Change:** Line 11: `nexus_descriptor.bin` → `gov_descriptor.bin`
- **Validation:** Grep confirms changes

#### Step 3.6: Update workspace members
- **File:** `/home/cmndcntrl/code/rust-nexus/Cargo.toml`
- **Change:** `"nexus-infra"` → `"gov-infra"`
- **Validation:** Workspace metadata confirms change

#### Step 3.7: Update nexus-agent dependency
- **File:** `/home/cmndcntrl/code/rust-nexus/nexus-agent/Cargo.toml`
- **Change:** Line 8: `nexus-infra = { path = "../nexus-infra" }` → `gov-infra = { path = "../gov-infra" }`
- **Validation:** Grep confirms change

#### Step 3.8: Update import statements in nexus-agent
- **Action:** Find and replace across all .rs files
- **Pattern:** `use nexus_infra::` → `use gov_infra::`
- **Pattern:** `nexus::v1::` → `gov::v1::`
- **Validation:** `grep -r "nexus_infra\|nexus::v1" nexus-agent/src` (should be empty)

#### Step 3.9: Update remaining dependent crates
- **Crates to check:** nexus-webui, nexus-recon, nexus-hybrid-exec
- **Action for each:** Update Cargo.toml and import statements

#### Step 3.10: Build validation
- **Action:** `cargo check --workspace`
- **Expected:** Success - proto regeneration should happen automatically
- **Commit:** "Rename nexus-infra to gov-infra including proto definitions"

---

### Step 4: Rename Agent Crate (nexus-agent → gov-agent)
**Objective:** Rename the agent crate and update build artifacts

#### Step 4.1: Rename directory
- **Action:** `mv nexus-agent gov-agent`
- **Validation:** Directory exists at new location

#### Step 4.2: Update package name
- **File:** `/home/cmndcntrl/code/rust-nexus/gov-agent/Cargo.toml`
- **Change:** `name = "nexus-agent"` → `name = "gov-agent"`

#### Step 4.3: Update build.rs hardcoded references
- **File:** `/home/cmndcntrl/code/rust-nexus/gov-agent/build.rs`
- **Changes:**
  - Line 8: `bofs/keylogger/nexus_keylogger.c` → `bofs/keylogger/gov_keylogger.c`
  - Line 56: `nexus_keylogger.c` → `gov_keylogger.c`
  - Line 57: `nexus_keylogger.obj` → `gov_keylogger.obj`
  - Line 68: `nexus_keylogger.obj` → `gov_keylogger.obj`
  - Line 68: `nexus_keylogger.o` → `gov_keylogger.o`
  - Line 102: `nexus_keylogger.o` → `gov_keylogger.o`
- **Validation:** Grep confirms all changes

#### Step 4.4: Rename BOF source file (if exists)
- **Action:** If file exists: `mv gov-agent/bofs/keylogger/nexus_keylogger.c gov-agent/bofs/keylogger/gov_keylogger.c`
- **Validation:** Check file location

#### Step 4.5: Update workspace members
- **File:** `/home/cmndcntrl/code/rust-nexus/Cargo.toml`
- **Change:** `"nexus-agent"` → `"gov-agent"`

#### Step 4.6: Update any references in other crates
- **Action:** Grep workspace for `nexus-agent` or `nexus_agent`
- **Fix:** Update any found references

#### Step 4.7: Build validation
- **Action:** `cargo check --workspace`
- **Expected:** Success
- **Commit:** "Rename nexus-agent to gov-agent"

---

### Step 5: Rename WebUI Crate (nexus-webui → gov-dashboard)
**Objective:** Rename the web interface crate

#### Step 5.1: Rename directory
- **Action:** `mv nexus-webui gov-dashboard`

#### Step 5.2: Update package name
- **File:** `/home/cmndcntrl/code/rust-nexus/gov-dashboard/Cargo.toml`
- **Change:** `name = "nexus-webui"` → `name = "gov-dashboard"`

#### Step 5.3: Update workspace members
- **File:** `/home/cmndcntrl/code/rust-nexus/Cargo.toml`
- **Change:** `"nexus-webui"` → `"gov-dashboard"`

#### Step 5.4: Update internal dependencies
- **Action:** Check and update any nexus-* dependencies in Cargo.toml to gov-* equivalents

#### Step 5.5: Update import statements
- **Action:** Update all use statements in src/ files
- **Patterns:** All nexus_* → gov_* imports

#### Step 5.6: Build validation
- **Action:** `cargo check --workspace`
- **Expected:** Success
- **Commit:** "Rename nexus-webui to gov-dashboard"

---

### Step 6: Rename Recon Crate (nexus-recon → gov-discovery)
**Objective:** Rename the reconnaissance crate

#### Step 6.1: Rename directory
- **Action:** `mv nexus-recon gov-discovery`

#### Step 6.2: Update package name
- **File:** `/home/cmndcntrl/code/rust-nexus/gov-discovery/Cargo.toml`
- **Change:** `name = "nexus-recon"` → `name = "gov-discovery"`

#### Step 6.3: Update workspace members
- **File:** `/home/cmndcntrl/code/rust-nexus/Cargo.toml`
- **Change:** `"nexus-recon"` → `"gov-discovery"`

#### Step 6.4: Update internal dependencies
- **Action:** Update all nexus-* dependencies to gov-*

#### Step 6.5: Update import statements
- **Action:** Update all use statements in src/

#### Step 6.6: Build validation
- **Action:** `cargo check --workspace`
- **Expected:** Success
- **Commit:** "Rename nexus-recon to gov-discovery"

---

### Step 7: Rename Hybrid Exec Crate (nexus-hybrid-exec → gov-collectors)
**Objective:** Rename the execution crate

#### Step 7.1: Rename directory
- **Action:** `mv nexus-hybrid-exec gov-collectors`

#### Step 7.2: Update package name
- **File:** `/home/cmndcntrl/code/rust-nexus/gov-collectors/Cargo.toml`
- **Change:** `name = "nexus-hybrid-exec"` → `name = "gov-collectors"`

#### Step 7.3: Update workspace members
- **File:** `/home/cmndcntrl/code/rust-nexus/Cargo.toml`
- **Change:** `"nexus-hybrid-exec"` → `"gov-collectors"`

#### Step 7.4: Update internal dependencies
- **Action:** Update all nexus-* dependencies to gov-*

#### Step 7.5: Update import statements
- **Action:** Update all use statements in src/

#### Step 7.6: Build validation
- **Action:** `cargo check --workspace`
- **Expected:** Success
- **Commit:** "Rename nexus-hybrid-exec to gov-collectors"

---

### Step 8: Update Configuration Files
**Objective:** Update all configuration files with new naming

#### Step 8.1: Update agent-windows.toml
- **File:** `/home/cmndcntrl/code/rust-nexus/config/agent-windows.toml`
- **Action:** Find and replace all nexus references with gov equivalents
- **Validation:** Grep confirms no nexus references remain

#### Step 8.2: Update agent-linux.toml
- **File:** `/home/cmndcntrl/code/rust-nexus/config/agent-linux.toml`
- **Action:** Find and replace all nexus references with gov equivalents
- **Validation:** Grep confirms no nexus references remain

#### Step 8.3: Rename nexus.toml
- **Action:** `mv nexus.toml gov.toml`
- **Update:** Replace all internal nexus references with gov

#### Step 8.4: Build validation
- **Action:** `cargo check --workspace`
- **Expected:** Success
- **Commit:** "Update configuration files to gov naming"

---

### Step 9: Final Workspace Validation
**Objective:** Comprehensive validation of all changes

#### Step 9.1: Full workspace build
- **Action:** `cargo build --workspace --all-features`
- **Expected:** Success
- **Duration:** May take several minutes

#### Step 9.2: Run workspace tests
- **Action:** `cargo test --workspace`
- **Expected:** All tests pass or expected failures only

#### Step 9.3: Verify no nexus references remain
- **Action:** `grep -r "nexus-\|nexus_\|nexus::" --include="*.rs" --include="*.toml" --exclude-dir=target .`
- **Expected:** Only acceptable references (comments, historical notes)
- **Exception:** May remain in git history, README history sections

#### Step 9.4: Verify all crates are recognized
- **Action:** `cargo tree --workspace`
- **Expected:** All gov-* crates appear in dependency tree

#### Step 9.5: Clean build from scratch
- **Action:**
  ```bash
  cargo clean
  cargo build --workspace
  ```
- **Expected:** Success

#### Step 9.6: Documentation review
- **Action:** Update any documentation files referencing old names
- **Files to check:**
  - README.md
  - ARCHITECTURE.md
  - Any docs/ files
  - CHANGELOG.md

#### Step 9.7: Final commit
- **Commit:** "Complete Phase 1: All nexus-* crates renamed to gov-*"
- **Tag:** `v0.2.0-gov-restructure-phase1`

---

## Phase 2: Create New Governance Crates

### Step 10: Create gov-compliance-engine
**Objective:** Create new compliance rule engine crate

#### Step 10.1: Create crate structure
- **Action:** `cargo new --lib gov-compliance-engine`
- **Location:** `/home/cmndcntrl/code/rust-nexus/`

#### Step 10.2: Add to workspace
- **File:** `/home/cmndcntrl/code/rust-nexus/Cargo.toml`
- **Change:** Add `"gov-compliance-engine"` to workspace.members

#### Step 10.3: Configure dependencies
- **File:** `/home/cmndcntrl/code/rust-nexus/gov-compliance-engine/Cargo.toml`
- **Add:** Dependencies on gov-common, workspace dependencies

#### Step 10.4: Create basic module structure
- **Files to create:**
  - `src/lib.rs` - Public API
  - `src/engine.rs` - Rule engine core
  - `src/rules.rs` - Rule definitions
  - `src/evaluator.rs` - Rule evaluation logic

#### Step 10.5: Build validation
- **Action:** `cargo check --workspace`
- **Expected:** Success
- **Commit:** "Add gov-compliance-engine crate"

---

### Step 11: Create gov-evidence
**Objective:** Create evidence collection and chain-of-custody crate

#### Step 11.1: Create crate
- **Action:** `cargo new --lib gov-evidence`

#### Step 11.2: Add to workspace
- **File:** Root Cargo.toml
- **Change:** Add to workspace.members

#### Step 11.3: Configure dependencies
- **Dependencies:** gov-common, chrono, uuid, serde

#### Step 11.4: Create module structure
- **Modules:**
  - `chain_of_custody.rs`
  - `evidence.rs`
  - `storage.rs`
  - `integrity.rs`

#### Step 11.5: Build validation
- **Action:** `cargo check --workspace`
- **Commit:** "Add gov-evidence crate"

---

### Step 12: Create gov-policy
**Objective:** Create policy management crate

#### Step 12.1: Create crate
- **Action:** `cargo new --lib gov-policy`

#### Step 12.2: Add to workspace
- **Change:** Add to workspace.members

#### Step 12.3: Configure dependencies
- **Dependencies:** gov-common, serde, serde_json

#### Step 12.4: Create module structure
- **Modules:**
  - `policy.rs`
  - `parser.rs`
  - `validator.rs`
  - `enforcer.rs`

#### Step 12.5: Build validation
- **Action:** `cargo check --workspace`
- **Commit:** "Add gov-policy crate"

---

### Step 13: Create gov-reporting
**Objective:** Create report generation crate

#### Step 13.1: Create crate
- **Action:** `cargo new --lib gov-reporting`

#### Step 13.2: Add to workspace
- **Change:** Add to workspace.members

#### Step 13.3: Configure dependencies
- **Dependencies:** gov-common, gov-evidence, chrono, serde

#### Step 13.4: Create module structure
- **Modules:**
  - `generator.rs`
  - `templates.rs`
  - `formatters.rs`
  - `exporters.rs`

#### Step 13.5: Build validation
- **Action:** `cargo check --workspace`
- **Commit:** "Add gov-reporting crate"

---

### Step 14: Create gov-api
**Objective:** Create REST API layer crate

#### Step 14.1: Create crate
- **Action:** `cargo new --lib gov-api`

#### Step 14.2: Add to workspace
- **Change:** Add to workspace.members

#### Step 14.3: Configure dependencies
- **Dependencies:**
  - gov-common
  - gov-infra
  - tokio
  - axum (add to workspace dependencies)
  - tower (add to workspace dependencies)

#### Step 14.4: Create module structure
- **Modules:**
  - `handlers.rs`
  - `middleware.rs`
  - `routes.rs`
  - `models.rs`

#### Step 14.5: Build validation
- **Action:** `cargo check --workspace`
- **Commit:** "Add gov-api crate"

---

### Step 15: Create gov-tenancy
**Objective:** Create multi-tenancy support crate

#### Step 15.1: Create crate
- **Action:** `cargo new --lib gov-tenancy`

#### Step 15.2: Add to workspace
- **Change:** Add to workspace.members

#### Step 15.3: Configure dependencies
- **Dependencies:** gov-common, uuid, serde

#### Step 15.4: Create module structure
- **Modules:**
  - `tenant.rs`
  - `isolation.rs`
  - `context.rs`
  - `permissions.rs`

#### Step 15.5: Build validation
- **Action:** `cargo check --workspace`
- **Commit:** "Add gov-tenancy crate"

---

### Step 16: Create gov-integrations
**Objective:** Create third-party integrations crate

#### Step 16.1: Create crate
- **Action:** `cargo new --lib gov-integrations`

#### Step 16.2: Add to workspace
- **Change:** Add to workspace.members

#### Step 16.3: Configure dependencies
- **Dependencies:**
  - gov-common
  - reqwest
  - serde
  - serde_json
  - tokio

#### Step 16.4: Create module structure
- **Modules:**
  - `connectors.rs`
  - `adapters.rs`
  - `webhooks.rs`
  - `oauth.rs`

#### Step 16.5: Build validation
- **Action:** `cargo check --workspace`
- **Commit:** "Add gov-integrations crate"

---

### Step 17: Final Phase 2 Validation
**Objective:** Validate complete workspace with all new crates

#### Step 17.1: Full workspace build
- **Action:** `cargo build --workspace --all-features`
- **Expected:** Success

#### Step 17.2: Dependency tree analysis
- **Action:** `cargo tree --workspace`
- **Validate:** All 13 crates present and properly linked

#### Step 17.3: Test all crates
- **Action:** `cargo test --workspace`
- **Expected:** All tests pass

#### Step 17.4: Documentation generation
- **Action:** `cargo doc --workspace --no-deps`
- **Expected:** Documentation generated for all crates

#### Step 17.5: Final commit and tag
- **Commit:** "Complete Phase 2: All new gov-* crates created"
- **Tag:** `v0.2.0-gov-restructure-complete`

---

## Phase 3: Documentation and Finalization

### Step 18: Update Documentation
**Objective:** Update all documentation to reflect new structure

#### Step 18.1: Update README.md
- **File:** `/home/cmndcntrl/code/rust-nexus/README.md`
- **Changes:**
  - Update project name references
  - Update workspace structure diagram
  - Update quick start commands
  - Update architecture overview

#### Step 18.2: Update ARCHITECTURE.md
- **File:** `/home/cmndcntrl/code/rust-nexus/ARCHITECTURE.md`
- **Changes:**
  - Update all crate names
  - Update dependency diagrams
  - Update module descriptions

#### Step 18.3: Create migration guide
- **File:** `/home/cmndcntrl/code/rust-nexus/docs/MIGRATION.md`
- **Content:**
  - Old name → New name mapping
  - Breaking changes
  - Update instructions for consumers

#### Step 18.4: Update CHANGELOG.md
- **File:** `/home/cmndcntrl/code/rust-nexus/CHANGELOG.md`
- **Add entry:**
  ```markdown
  ## [0.2.0] - 2025-12-21
  ### Changed
  - BREAKING: Renamed all nexus-* crates to gov-*
  - nexus-common → gov-common
  - nexus-infra → gov-infra
  - nexus-agent → gov-agent
  - nexus-webui → gov-dashboard
  - nexus-recon → gov-discovery
  - nexus-hybrid-exec → gov-collectors

  ### Added
  - gov-compliance-engine: Compliance rule engine
  - gov-evidence: Evidence collection and chain-of-custody
  - gov-policy: Policy management and enforcement
  - gov-reporting: Report generation
  - gov-api: REST API layer
  - gov-tenancy: Multi-tenancy support
  - gov-integrations: Third-party integrations

  ### Removed
  - nexus-web-comms: Obsolete, functionality merged elsewhere
  ```

#### Step 18.5: Commit documentation updates
- **Commit:** "Update documentation for gov-* restructuring"

---

## Rollback Procedures

### Emergency Rollback Steps
If at any point a step fails and cannot be resolved:

1. **Identify the last successful commit:** `git log --oneline`
2. **Hard reset to that commit:** `git reset --hard <commit-hash>`
3. **Verify workspace state:** `cargo check --workspace`
4. **Document the failure:** Add notes to this file about what went wrong
5. **Analyze before retry:** Understand root cause before attempting again

### Partial Rollback
If only one crate rename fails:

1. **Revert that crate's directory name:** `mv gov-name nexus-name`
2. **Revert Cargo.toml changes:** Git checkout specific files
3. **Revert dependent references:** Restore original import statements
4. **Validate build:** Ensure workspace builds
5. **Recommit stable state:** Commit the rollback

---

## Risk Mitigation

### Pre-Execution Checklist
- [ ] Full workspace backup created
- [ ] Git working tree is clean (`git status`)
- [ ] All existing tests pass (`cargo test --workspace`)
- [ ] All crates build successfully (`cargo build --workspace`)
- [ ] Branch created for restructuring work
- [ ] This plan reviewed and approved

### During Execution Safeguards
- [ ] Only one step in progress at a time
- [ ] Build validation after each step
- [ ] Commit after each successful validation
- [ ] Document any deviations from plan
- [ ] Keep terminal output logs

### Post-Step Validation Checklist
- [ ] `cargo check --workspace` passes
- [ ] No unexpected warnings
- [ ] Git status shows expected changes only
- [ ] Grep confirms reference updates
- [ ] Commit message is descriptive

---

## Success Criteria

### Phase 1 Complete When:
- [ ] All 6 nexus-* crates renamed to gov-*
- [ ] Zero references to nexus-* in active code (excluding docs/history)
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` passes
- [ ] Proto definitions use gov.v1 package
- [ ] All configuration files updated

### Phase 2 Complete When:
- [ ] All 7 new gov-* crates created
- [ ] Each new crate has basic module structure
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo doc --workspace` generates docs
- [ ] Dependency tree shows all 13 crates

### Phase 3 Complete When:
- [ ] All documentation updated
- [ ] Migration guide created
- [ ] CHANGELOG.md updated
- [ ] README.md reflects new structure
- [ ] Tagged release created

---

## Estimated Timeline

### Time Estimates (Baby Steps™ Pace)
- **Step 1 (Delete nexus-web-comms):** 10 minutes
- **Step 2 (Rename nexus-common):** 30 minutes
- **Step 3 (Rename nexus-infra):** 45 minutes (proto changes)
- **Step 4 (Rename nexus-agent):** 30 minutes
- **Step 5 (Rename nexus-webui):** 20 minutes
- **Step 6 (Rename nexus-recon):** 20 minutes
- **Step 7 (Rename nexus-hybrid-exec):** 20 minutes
- **Step 8 (Update config files):** 15 minutes
- **Step 9 (Phase 1 validation):** 30 minutes
- **Steps 10-16 (Create new crates):** 20 minutes each = 140 minutes
- **Step 17 (Phase 2 validation):** 30 minutes
- **Step 18 (Documentation):** 45 minutes

**Total Estimated Time:** 6.5 hours (with validation and breaks)

### Recommended Execution Schedule
- **Session 1 (2 hours):** Steps 1-4
- **Session 2 (1.5 hours):** Steps 5-9
- **Session 3 (2.5 hours):** Steps 10-16
- **Session 4 (1 hour):** Steps 17-18

---

## Notes and Observations

### Discovery Notes
- nexus-agent build.rs has hardcoded BOF file names that must be updated
- Proto package name change will trigger full regeneration
- nexus-infra build.rs references proto file path explicitly
- Multiple config files reference old naming

### Potential Challenges
1. **Proto regeneration:** May cause temporary compilation errors until all imports updated
2. **BOF file renames:** Must coordinate .c file and build.rs changes
3. **Import statements:** Large number of files may have nexus references
4. **Config file format:** May have nested references requiring careful updates
5. **Git history:** Old names will remain in history (acceptable)

### Dependencies to Monitor
```
gov-agent → gov-common, gov-infra
gov-dashboard → gov-common, gov-infra
gov-discovery → gov-common
gov-collectors → gov-common
gov-api → gov-common, gov-infra, gov-tenancy
gov-reporting → gov-common, gov-evidence, gov-policy
```

---

## Appendix: Command Reference

### Useful Grep Commands
```bash
# Find all nexus references in Rust files
grep -r "nexus_\|nexus-\|nexus::" --include="*.rs" --exclude-dir=target .

# Find all nexus references in TOML files
grep -r "nexus" --include="*.toml" --exclude-dir=target .

# Find specific crate usage
grep -r "use nexus_common::" --include="*.rs" .

# Verify workspace members
cargo metadata --no-deps | jq '.workspace_members'

# Check for proto package references
grep -r "nexus.v1\|nexus::v1" --include="*.rs" .
```

### Useful Cargo Commands
```bash
# Check workspace without building
cargo check --workspace

# Build entire workspace
cargo build --workspace

# Run all tests
cargo test --workspace

# Generate docs
cargo doc --workspace --no-deps --open

# View dependency tree
cargo tree --workspace

# Clean build artifacts
cargo clean

# Update Cargo.lock
cargo update
```

### Git Commands for This Migration
```bash
# Create migration branch
git checkout -b feature/gov-restructure

# Stage specific files
git add <file>

# Commit with descriptive message
git commit -m "Step N: <description>"

# Tag important milestones
git tag -a v0.2.0-phase1 -m "Phase 1 complete"

# View recent commits
git log --oneline -10

# Emergency rollback
git reset --hard HEAD~1
```

---

## Implementation Log

### Execution Log Template
**Date:** [Date]
**Executor:** [Name]
**Session:** [Session number]
**Time Started:** [Time]
**Time Ended:** [Time]

#### Steps Completed
- [ ] Step X.Y: [Description] - [Status: Success/Failed/Skipped]
  - **Duration:** [Time]
  - **Issues:** [Any issues encountered]
  - **Resolution:** [How resolved]
  - **Commit:** [Commit hash]

#### Deviations from Plan
[Document any changes to the planned approach]

#### Lessons Learned
[Document insights for future similar work]

---

## Sign-Off

### Phase 1 Sign-Off
- [ ] All renames completed successfully
- [ ] Full workspace build passes
- [ ] All tests pass
- [ ] Documentation updated
- [ ] Signed off by: _________________ Date: _________

### Phase 2 Sign-Off
- [ ] All new crates created
- [ ] Full workspace build passes
- [ ] Basic functionality verified
- [ ] Documentation created
- [ ] Signed off by: _________________ Date: _________

### Phase 3 Sign-Off
- [ ] All documentation complete
- [ ] Migration guide published
- [ ] Release tagged
- [ ] Stakeholders notified
- [ ] Signed off by: _________________ Date: _________

---

**End of Implementation Plan**

*This document follows the Baby Steps™ Methodology - The Process is the Product*
