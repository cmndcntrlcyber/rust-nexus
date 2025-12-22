# Proto and gRPC Transformation Implementation Plan

## Overview
This document provides a Baby Steps™ implementation plan to transform `nexus.proto` to `gov.proto`, removing offensive security capabilities and replacing them with governance, risk, and compliance (GRC) functionality.

**Source File:** `/home/cmndcntrl/code/rust-nexus/nexus-infra/proto/nexus.proto`
**Target File:** `/home/cmndcntrl/code/rust-nexus/nexus-infra/proto/gov.proto`

**Remember: The process is the product. Each step must be completed fully before moving to the next.**

---

## Phase 1: Proto File Structure Setup

### Step 1.1: Create Base Gov Proto File
**Goal:** Create new gov.proto with basic structure and package declaration

**Actions:**
1. Create new file: `/home/cmndcntrl/code/rust-nexus/nexus-infra/proto/gov.proto`
2. Copy header section from nexus.proto (lines 1-6):
   - syntax declaration
   - package name (change to `gov.v1`)
   - imports for google/protobuf/timestamp.proto and empty.proto

**Validation:**
- File exists at correct location
- Syntax is proto3
- Package is named gov.v1
- Required imports are present

**Deliverable:** Initial gov.proto file with header only

---

### Step 1.2: Copy Core Infrastructure Messages
**Goal:** Copy non-offensive infrastructure messages unchanged

**Actions:**
1. Copy these message definitions from nexus.proto to gov.proto:
   - RegistrationRequest (lines 29-40)
   - RegistrationResponse (lines 42-48)
   - HeartbeatRequest (lines 51-55)
   - HeartbeatResponse (lines 57-62)
   - SystemStatus (lines 65-71)
   - NetworkInterface (lines 73-78)
   - ProcessInfo (lines 80-87)
   - AgentInfoRequest (lines 297-299)
   - AgentInfoResponse (lines 301-308)

**Validation:**
- All messages copied with correct formatting
- No modifications to field names or numbers
- protoc can parse the file without errors

**Deliverable:** gov.proto with core infrastructure messages

---

### Step 1.3: Copy Task Management Infrastructure
**Goal:** Copy task management structure (without task types)

**Actions:**
1. Copy these messages to gov.proto:
   - TaskRequest (lines 90-93)
   - Task (lines 95-105)
   - TaskStatus (lines 147-152)
   - TaskExecutionStatus enum (lines 154-162)
   - TaskResult (lines 164-175)
   - TaskResultResponse (lines 177-180)

**Validation:**
- All task infrastructure messages present
- TaskType enum NOT included yet (will be replaced)
- protoc validation passes

**Deliverable:** gov.proto with task management infrastructure

---

### Step 1.4: Copy File Operations Messages
**Goal:** Add legitimate file operation message definitions

**Actions:**
1. Copy these messages to gov.proto:
   - FileChunk (lines 204-210)
   - FileUploadResponse (lines 212-216)
   - FileDownloadRequest (lines 218-223)

**Validation:**
- File operation messages present
- No modifications to structure
- protoc validation passes

**Deliverable:** gov.proto with file operations support

---

### Step 1.5: Copy Configuration Messages
**Goal:** Add configuration management messages

**Actions:**
1. Copy these messages to gov.proto:
   - ConfigUpdate (lines 281-287)
   - CertificateUpdate (lines 289-294)

**Validation:**
- Configuration messages present
- protoc validation passes

**Deliverable:** gov.proto with configuration management

---

## Phase 2: Remove Offensive Task Types

### Step 2.1: Document Original Task Types for Removal
**Goal:** Create explicit record of what is being removed

**Actions:**
1. Create removal inventory in this document
2. List each offensive task type with:
   - Enum value name
   - Numeric value
   - Security concern category

**Validation:**
- All offensive types documented
- Rationale for removal clear

**Deliverable:** Complete removal inventory section (below)

#### Offensive Task Types Removal Inventory

**Advanced Execution Methods (20-27):**
- TASK_TYPE_FIBER_SHELLCODE = 20 (arbitrary code execution)
- TASK_TYPE_FIBER_HOLLOWING = 21 (process hollowing attack)
- TASK_TYPE_PROCESS_INJECTION = 22 (code injection attack)
- TASK_TYPE_DLL_INJECTION = 23 (DLL hijacking)
- TASK_TYPE_APC_INJECTION = 24 (async procedure call injection)
- TASK_TYPE_EARLY_BIRD_INJECTION = 25 (early process injection)
- TASK_TYPE_BOF_EXECUTION = 26 (beacon object file execution)
- TASK_TYPE_COFF_LOADING = 27 (common object file format loading)

**Reconnaissance (41-45):**
- TASK_TYPE_CREDENTIAL_HARVESTING = 41 (credential theft)
- TASK_TYPE_BROWSER_DATA_EXTRACTION = 42 (data exfiltration)
- TASK_TYPE_KEYLOGGER_START = 44 (keylogging)
- TASK_TYPE_KEYLOGGER_STOP = 45 (keylogging)

**Cleanup (50-52):**
- TASK_TYPE_SELF_DESTRUCT = 50 (evidence destruction)
- TASK_TYPE_LOG_CLEANING = 51 (evidence tampering)
- TASK_TYPE_ARTIFACT_REMOVAL = 52 (evidence removal)

---

### Step 2.2: Create New TaskType Enum - Core Operations
**Goal:** Create TaskType enum with only legitimate operations

**Actions:**
1. Create TaskType enum in gov.proto with:
   - TASK_TYPE_UNSPECIFIED = 0
   - TASK_TYPE_SHELL_COMMAND = 1
   - TASK_TYPE_POWERSHELL_COMMAND = 2
   - TASK_TYPE_FILE_UPLOAD = 3
   - TASK_TYPE_FILE_DOWNLOAD = 4
   - TASK_TYPE_DIRECTORY_LISTING = 5
   - TASK_TYPE_PROCESS_LIST = 6
   - TASK_TYPE_SYSTEM_INFO = 7
   - TASK_TYPE_NETWORK_INFO = 8
   - TASK_TYPE_REGISTRY_QUERY = 9
   - TASK_TYPE_SERVICE_CONTROL = 11

2. Omit all offensive types (20-27, 41-42, 44-45, 50-52)
3. Keep legitimate persistence/reconnaissance:
   - TASK_TYPE_REGISTRY_PERSISTENCE = 30
   - TASK_TYPE_SERVICE_PERSISTENCE = 31
   - TASK_TYPE_TASK_SCHEDULER_PERSISTENCE = 32
   - TASK_TYPE_STARTUP_PERSISTENCE = 33
   - TASK_TYPE_NETWORK_SCAN = 40
   - TASK_TYPE_SCREEN_CAPTURE = 43

**Validation:**
- No offensive task types present
- Numeric values unchanged for retained types
- protoc validation passes
- Enum properly formatted

**Deliverable:** TaskType enum with only legitimate operations

---

## Phase 3: Add Compliance Task Types

### Step 3.1: Add SCAP/OVAL Compliance Tasks
**Goal:** Add security compliance automation tasks

**Actions:**
1. Add to TaskType enum:
   ```protobuf
   // Security Content Automation Protocol (SCAP) compliance
   TASK_TYPE_SCAP_OVAL_CHECK = 100;
   TASK_TYPE_CIS_BENCHMARK = 101;
   ```

**Validation:**
- New task types added
- Numeric range 100+ used to avoid conflicts
- protoc validation passes

**Deliverable:** TaskType enum with SCAP compliance support

---

### Step 3.2: Add Registry and File Audit Tasks
**Goal:** Add audit and integrity verification tasks

**Actions:**
1. Add to TaskType enum:
   ```protobuf
   // Audit and integrity tasks
   TASK_TYPE_REGISTRY_AUDIT = 102;
   TASK_TYPE_FILE_INTEGRITY = 103;
   ```

**Validation:**
- Audit task types added
- Sequential numbering maintained
- protoc validation passes

**Deliverable:** TaskType enum with audit capabilities

---

### Step 3.3: Add Service and Access Review Tasks
**Goal:** Add operational audit tasks

**Actions:**
1. Add to TaskType enum:
   ```protobuf
   // Operational audits
   TASK_TYPE_SERVICE_AUDIT = 104;
   TASK_TYPE_USER_ACCESS_REVIEW = 105;
   ```

**Validation:**
- Access review tasks added
- protoc validation passes

**Deliverable:** TaskType enum with access review support

---

### Step 3.4: Add Evidence Collection Tasks
**Goal:** Add forensically sound evidence collection

**Actions:**
1. Add to TaskType enum:
   ```protobuf
   // Evidence collection (forensically sound)
   TASK_TYPE_COLLECT_EVIDENCE = 200;
   TASK_TYPE_SCREENSHOT = 201;
   TASK_TYPE_CONFIG_EXPORT = 202;
   ```

**Validation:**
- Evidence tasks use 200+ range
- Clear separation from compliance tasks
- protoc validation passes

**Deliverable:** TaskType enum with evidence collection

---

### Step 3.5: Add Asset Management Tasks
**Goal:** Add asset inventory and software scanning

**Actions:**
1. Add to TaskType enum:
   ```protobuf
   // Asset management
   TASK_TYPE_ASSET_INVENTORY = 300;
   TASK_TYPE_SOFTWARE_SCAN = 301;
   ```

**Validation:**
- Asset tasks use 300+ range
- Clear functional grouping
- protoc validation passes

**Deliverable:** Complete TaskType enum with all compliance capabilities

---

## Phase 4: Create New Compliance Messages

### Step 4.1: Create ComplianceCheck Message
**Goal:** Define structure for compliance check requests

**Actions:**
1. Add ComplianceCheck message to gov.proto:
   ```protobuf
   message ComplianceCheck {
     string check_id = 1;
     string standard = 2;           // e.g., "CIS", "NIST", "PCI-DSS"
     string control_id = 3;         // e.g., "1.1.1", "AC-2"
     string description = 4;
     map<string, string> parameters = 5;
     google.protobuf.Timestamp scheduled_time = 6;
   }
   ```

**Validation:**
- Message properly formatted
- Field numbers sequential
- Standard compliance framework fields present
- protoc validation passes

**Deliverable:** ComplianceCheck message definition

---

### Step 4.2: Create CheckResult Message
**Goal:** Define structure for compliance check results

**Actions:**
1. Add CheckResult message to gov.proto:
   ```protobuf
   message CheckResult {
     string check_id = 1;
     CheckStatus status = 2;
     string finding = 3;
     string evidence_reference = 4;
     repeated string affected_resources = 5;
     string remediation_guidance = 6;
     google.protobuf.Timestamp checked_at = 7;
     map<string, string> metadata = 8;
   }

   enum CheckStatus {
     CHECK_STATUS_UNSPECIFIED = 0;
     CHECK_STATUS_PASS = 1;
     CHECK_STATUS_FAIL = 2;
     CHECK_STATUS_NOT_APPLICABLE = 3;
     CHECK_STATUS_ERROR = 4;
     CHECK_STATUS_MANUAL_REVIEW = 5;
   }
   ```

**Validation:**
- CheckResult message complete
- CheckStatus enum included
- All standard result fields present
- protoc validation passes

**Deliverable:** CheckResult message and enum

---

### Step 4.3: Create ControlAssessment Message
**Goal:** Define aggregate control assessment structure

**Actions:**
1. Add ControlAssessment message to gov.proto:
   ```protobuf
   message ControlAssessment {
     string assessment_id = 1;
     string framework = 2;           // e.g., "NIST CSF", "ISO 27001"
     repeated CheckResult checks = 3;
     AssessmentSummary summary = 4;
     google.protobuf.Timestamp assessment_start = 5;
     google.protobuf.Timestamp assessment_end = 6;
     string assessor_id = 7;
   }

   message AssessmentSummary {
     uint32 total_checks = 1;
     uint32 passed = 2;
     uint32 failed = 3;
     uint32 not_applicable = 4;
     uint32 errors = 5;
     double compliance_percentage = 6;
   }
   ```

**Validation:**
- ControlAssessment message complete
- Nested AssessmentSummary included
- Framework-agnostic design
- protoc validation passes

**Deliverable:** ControlAssessment message structure

---

### Step 4.4: Create Evidence Collection Messages
**Goal:** Define forensically sound evidence collection

**Actions:**
1. Add Evidence messages to gov.proto:
   ```protobuf
   message Evidence {
     string evidence_id = 1;
     EvidenceType type = 2;
     string description = 3;
     google.protobuf.Timestamp collected_at = 4;
     string collector_id = 5;
     string source_system = 6;
     ChainOfCustody chain = 7;
     map<string, string> metadata = 8;
   }

   enum EvidenceType {
     EVIDENCE_TYPE_UNSPECIFIED = 0;
     EVIDENCE_TYPE_CONFIGURATION = 1;
     EVIDENCE_TYPE_LOG_FILE = 2;
     EVIDENCE_TYPE_SCREENSHOT = 3;
     EVIDENCE_TYPE_REGISTRY_EXPORT = 4;
     EVIDENCE_TYPE_FILE_HASH = 5;
     EVIDENCE_TYPE_NETWORK_CONFIG = 6;
     EVIDENCE_TYPE_SYSTEM_STATE = 7;
   }
   ```

**Validation:**
- Evidence message structure sound
- EvidenceType enum comprehensive
- Chain of custody reference included
- protoc validation passes

**Deliverable:** Evidence message definitions

---

### Step 4.5: Create EvidenceChunk Message
**Goal:** Support streaming large evidence files

**Actions:**
1. Add EvidenceChunk message to gov.proto:
   ```protobuf
   message EvidenceChunk {
     string evidence_id = 1;
     bytes data = 2;
     uint64 offset = 3;
     uint64 total_size = 4;
     string chunk_hash = 5;         // SHA256 of this chunk
     string final_hash = 6;         // SHA256 of complete evidence
     uint32 chunk_sequence = 7;
   }
   ```

**Validation:**
- Chunk message supports streaming
- Hash fields for integrity verification
- Sequence tracking included
- protoc validation passes

**Deliverable:** EvidenceChunk message for streaming

---

### Step 4.6: Create ChainOfCustody Message
**Goal:** Define audit trail for evidence handling

**Actions:**
1. Add ChainOfCustody message to gov.proto:
   ```protobuf
   message ChainOfCustody {
     repeated CustodyEvent events = 1;
     string initial_custodian = 2;
     google.protobuf.Timestamp initial_timestamp = 3;
   }

   message CustodyEvent {
     string custodian_id = 1;
     string action = 2;              // "collected", "transferred", "analyzed"
     google.protobuf.Timestamp timestamp = 3;
     string location = 4;
     string notes = 5;
     string digital_signature = 6;
   }
   ```

**Validation:**
- Chain of custody trackable
- Event log structure complete
- Digital signature support
- protoc validation passes

**Deliverable:** ChainOfCustody message structure

---

### Step 4.7: Create AssetInventory Message
**Goal:** Define asset tracking and inventory

**Actions:**
1. Add AssetInventory message to gov.proto:
   ```protobuf
   message AssetInventory {
     string inventory_id = 1;
     google.protobuf.Timestamp scan_time = 2;
     repeated AssetItem assets = 3;
     string agent_id = 4;
   }

   message AssetItem {
     string asset_id = 1;
     AssetType asset_type = 2;
     string name = 3;
     string version = 4;
     string vendor = 5;
     string install_date = 6;
     string location = 7;           // File path or registry key
     map<string, string> properties = 8;
   }

   enum AssetType {
     ASSET_TYPE_UNSPECIFIED = 0;
     ASSET_TYPE_SOFTWARE = 1;
     ASSET_TYPE_HARDWARE = 2;
     ASSET_TYPE_SERVICE = 3;
     ASSET_TYPE_CERTIFICATE = 4;
     ASSET_TYPE_USER_ACCOUNT = 5;
     ASSET_TYPE_NETWORK_DEVICE = 6;
   }
   ```

**Validation:**
- Asset inventory structure complete
- AssetType enum comprehensive
- Flexible properties map included
- protoc validation passes

**Deliverable:** AssetInventory message definitions

---

### Step 4.8: Create ConfigurationBaseline Message
**Goal:** Define configuration baseline tracking

**Actions:**
1. Add ConfigurationBaseline message to gov.proto:
   ```protobuf
   message ConfigurationBaseline {
     string baseline_id = 1;
     string baseline_name = 2;
     string description = 3;
     google.protobuf.Timestamp created_at = 4;
     repeated ConfigurationItem items = 5;
     string approved_by = 6;
     string version = 7;
   }

   message ConfigurationItem {
     string item_id = 1;
     string category = 2;           // "registry", "file", "service", "policy"
     string key_path = 3;           // Registry key, file path, etc.
     string expected_value = 4;
     string actual_value = 5;
     bool compliant = 6;
     string notes = 7;
   }
   ```

**Validation:**
- Configuration baseline structure complete
- Item comparison fields present
- Compliance tracking included
- protoc validation passes

**Deliverable:** ConfigurationBaseline message definitions

---

## Phase 5: Update Artifact Types

### Step 5.1: Review Current ArtifactType Enum
**Goal:** Identify offensive artifact types for removal

**Actions:**
1. Review current ArtifactType enum (lines 189-201)
2. Identify offensive types:
   - ARTIFACT_TYPE_CREDENTIAL_DUMP = 3 (credential theft)
   - ARTIFACT_TYPE_PROCESS_DUMP = 4 (potential malware analysis)
   - ARTIFACT_TYPE_MEMORY_DUMP = 5 (potential malware analysis)

**Validation:**
- Offensive types identified
- Legitimate types preserved

**Deliverable:** Artifact type removal list

---

### Step 5.2: Create Updated ArtifactType Enum
**Goal:** Replace with compliance-focused artifact types

**Actions:**
1. Copy TaskArtifact message to gov.proto
2. Create new ArtifactType enum:
   ```protobuf
   enum ArtifactType {
     ARTIFACT_TYPE_UNSPECIFIED = 0;
     ARTIFACT_TYPE_SCREENSHOT = 1;
     ARTIFACT_TYPE_LOG_FILE = 2;
     ARTIFACT_TYPE_COMPLIANCE_REPORT = 3;
     ARTIFACT_TYPE_CONFIGURATION_EXPORT = 4;
     ARTIFACT_TYPE_REGISTRY_EXPORT = 5;
     ARTIFACT_TYPE_NETWORK_CAPTURE = 6;
     ARTIFACT_TYPE_FILE_CONTENT = 7;
     ARTIFACT_TYPE_CONFIG_DATA = 8;
     ARTIFACT_TYPE_EVIDENCE_PACKAGE = 9;
     ARTIFACT_TYPE_AUDIT_LOG = 10;
     ARTIFACT_TYPE_BASELINE_SNAPSHOT = 11;
     ARTIFACT_TYPE_CUSTOM = 12;
   }
   ```

**Validation:**
- No offensive artifact types
- Compliance types added
- protoc validation passes

**Deliverable:** Updated ArtifactType enum

---

## Phase 6: Remove Offensive RPC Methods

### Step 6.1: Document RPC Methods for Removal
**Goal:** Identify offensive RPC methods

**Actions:**
1. List offensive RPC methods from NexusC2 service:
   - ExecuteShellcode (line 24)
   - ExecuteBOF (line 25)

2. List associated messages to NOT copy:
   - ShellcodeRequest (lines 226-233)
   - ShellcodeExecutionMethod enum (lines 235-243)
   - ShellcodeResponse (lines 245-249)
   - BOFRequest (lines 252-258)
   - BOFArgument (lines 260-263)
   - BOFArgumentType enum (lines 265-272)
   - BOFResponse (lines 274-278)

**Validation:**
- All offensive RPC methods identified
- Associated messages cataloged

**Deliverable:** RPC removal documentation

---

### Step 6.2: Create GovernanceService RPC Definition
**Goal:** Define new service with only legitimate methods

**Actions:**
1. Create GovernanceService in gov.proto:
   ```protobuf
   service GovernanceService {
     // Agent registration and management
     rpc RegisterAgent(RegistrationRequest) returns (RegistrationResponse);
     rpc Heartbeat(HeartbeatRequest) returns (HeartbeatResponse);
     rpc GetAgentInfo(AgentInfoRequest) returns (AgentInfoResponse);

     // Task management
     rpc GetTasks(TaskRequest) returns (stream Task);
     rpc SubmitTaskResult(TaskResult) returns (TaskResultResponse);

     // File operations
     rpc UploadFile(stream FileChunk) returns (FileUploadResponse);
     rpc DownloadFile(FileDownloadRequest) returns (stream FileChunk);
   }
   ```

2. Note: ExecuteShellcode and ExecuteBOF NOT included

**Validation:**
- Only legitimate RPC methods present
- Method signatures match message definitions
- No offensive capabilities
- protoc validation passes

**Deliverable:** GovernanceService RPC definition

---

## Phase 7: Add New Compliance RPC Methods

### Step 7.1: Add Compliance Check RPC Methods
**Goal:** Add methods for compliance automation

**Actions:**
1. Add to GovernanceService:
   ```protobuf
   // Compliance operations
   rpc SubmitComplianceCheck(ComplianceCheck) returns (CheckResult);
   rpc GetControlAssessment(AssessmentRequest) returns (ControlAssessment);
   rpc StreamCheckResults(stream CheckResult) returns (AssessmentSummary);
   ```

2. Add supporting message:
   ```protobuf
   message AssessmentRequest {
     string agent_id = 1;
     string framework = 2;
     repeated string control_ids = 3;
   }
   ```

**Validation:**
- Compliance RPC methods added
- Supporting messages defined
- protoc validation passes

**Deliverable:** Compliance RPC methods in service

---

### Step 7.2: Add Evidence Collection RPC Methods
**Goal:** Add methods for evidence management

**Actions:**
1. Add to GovernanceService:
   ```protobuf
   // Evidence collection
   rpc CollectEvidence(EvidenceRequest) returns (Evidence);
   rpc StreamEvidence(stream EvidenceChunk) returns (EvidenceReceipt);
   rpc GetChainOfCustody(EvidenceQuery) returns (ChainOfCustody);
   ```

2. Add supporting messages:
   ```protobuf
   message EvidenceRequest {
     string agent_id = 1;
     EvidenceType type = 2;
     string target_path = 3;
     map<string, string> parameters = 4;
   }

   message EvidenceReceipt {
     string evidence_id = 1;
     bool success = 2;
     string storage_location = 3;
     string final_hash = 4;
     google.protobuf.Timestamp received_at = 5;
   }

   message EvidenceQuery {
     string evidence_id = 1;
     string agent_id = 2;
   }
   ```

**Validation:**
- Evidence RPC methods added
- Request/response messages complete
- protoc validation passes

**Deliverable:** Evidence collection RPC methods

---

### Step 7.3: Add Asset Management RPC Methods
**Goal:** Add methods for asset tracking

**Actions:**
1. Add to GovernanceService:
   ```protobuf
   // Asset management
   rpc ScanAssets(AssetScanRequest) returns (AssetInventory);
   rpc GetConfigurationBaseline(BaselineRequest) returns (ConfigurationBaseline);
   rpc CompareToBaseline(BaselineComparisonRequest) returns (BaselineComparisonResult);
   ```

2. Add supporting messages:
   ```protobuf
   message AssetScanRequest {
     string agent_id = 1;
     repeated AssetType scan_types = 2;
     bool include_details = 3;
   }

   message BaselineRequest {
     string baseline_id = 1;
     string agent_id = 2;
   }

   message BaselineComparisonRequest {
     string agent_id = 1;
     string baseline_id = 2;
   }

   message BaselineComparisonResult {
     string comparison_id = 1;
     ConfigurationBaseline baseline = 2;
     repeated ConfigurationItem deviations = 3;
     uint32 total_items = 4;
     uint32 compliant_items = 5;
     uint32 non_compliant_items = 6;
     google.protobuf.Timestamp compared_at = 7;
   }
   ```

**Validation:**
- Asset RPC methods added
- Baseline comparison support complete
- protoc validation passes

**Deliverable:** Asset management RPC methods

---

## Phase 8: Validation and Documentation

### Step 8.1: Complete Proto Validation
**Goal:** Ensure gov.proto is fully valid

**Actions:**
1. Run protoc validation:
   ```bash
   protoc --proto_path=/home/cmndcntrl/code/rust-nexus/nexus-infra/proto \
          --descriptor_set_out=/dev/null \
          /home/cmndcntrl/code/rust-nexus/nexus-infra/proto/gov.proto
   ```

2. Verify no errors or warnings

**Validation:**
- protoc exits with status 0
- No syntax errors
- No undefined message references
- All field numbers unique within messages

**Deliverable:** Validated gov.proto file

---

### Step 8.2: Generate Proto Documentation
**Goal:** Create human-readable documentation

**Actions:**
1. Use protoc-gen-doc to generate documentation:
   ```bash
   protoc --proto_path=/home/cmndcntrl/code/rust-nexus/nexus-infra/proto \
          --doc_out=/home/cmndcntrl/code/rust-nexus/docs/proto \
          --doc_opt=markdown,gov-proto-reference.md \
          /home/cmndcntrl/code/rust-nexus/nexus-infra/proto/gov.proto
   ```

**Validation:**
- Documentation generated successfully
- All messages documented
- All RPC methods described
- Output is readable markdown

**Deliverable:** `/home/cmndcntrl/code/rust-nexus/docs/proto/gov-proto-reference.md`

---

### Step 8.3: Create Migration Mapping Document
**Goal:** Document nexus.proto to gov.proto mapping

**Actions:**
1. Create file: `/home/cmndcntrl/code/rust-nexus/docs/proto/nexus-to-gov-mapping.md`
2. Include sections:
   - Removed task types with rationale
   - Removed RPC methods with rationale
   - Added task types with purpose
   - Added messages with purpose
   - Field number mappings for retained messages

**Validation:**
- All changes documented
- Rationale clear for each change
- Complete reference for developers

**Deliverable:** Migration mapping document

---

### Step 8.4: Verify Compatibility
**Goal:** Ensure no breaking changes to retained functionality

**Actions:**
1. Compare field numbers for retained messages:
   - RegistrationRequest
   - HeartbeatRequest/Response
   - Task and TaskResult
   - FileChunk

2. Verify field numbers unchanged
3. Verify field types unchanged
4. Verify message names unchanged (except service name)

**Validation:**
- No field number conflicts
- Retained messages wire-compatible
- Only removals and additions made

**Deliverable:** Compatibility verification checklist

---

## Phase 9: Integration Points

### Step 9.1: Identify Rust Code Dependencies
**Goal:** Find all Rust code importing nexus.proto

**Actions:**
1. Search for proto imports:
   ```bash
   grep -r "nexus.proto" /home/cmndcntrl/code/rust-nexus --include="*.rs"
   grep -r "nexus::v1" /home/cmndcntrl/code/rust-nexus --include="*.rs"
   grep -r "nexus_c2" /home/cmndcntrl/code/rust-nexus --include="*.rs"
   ```

2. Create list of affected files

**Validation:**
- All proto consumers identified
- File paths documented

**Deliverable:** List of files requiring updates in Phase 10

---

### Step 9.2: Identify Build Configuration Dependencies
**Goal:** Find build files referencing proto compilation

**Actions:**
1. Search for proto build configs:
   ```bash
   grep -r "nexus.proto" /home/cmndcntrl/code/rust-nexus --include="build.rs"
   grep -r "nexus.proto" /home/cmndcntrl/code/rust-nexus --include="Cargo.toml"
   ```

2. Document build.rs files that compile protos

**Validation:**
- All build configurations found
- Proto compilation steps identified

**Deliverable:** Build configuration update list

---

### Step 9.3: Document Required Rust Changes
**Goal:** Create checklist for Rust code updates

**Actions:**
1. For each identified file, document:
   - Current proto import
   - Required new import (gov.v1)
   - Service name changes (NexusC2 -> GovernanceService)
   - Message type changes (if any)
   - Removed functionality to handle

2. Prioritize by dependency order

**Validation:**
- Complete change list
- Dependencies ordered correctly

**Deliverable:** Rust code update checklist for Phase 10

---

## Phase 10: Proto File Finalization

### Step 10.1: Add Proto File Header Comments
**Goal:** Document purpose and transformation

**Actions:**
1. Add comprehensive header to gov.proto:
   ```protobuf
   // gov.proto - Governance, Risk, and Compliance Protocol
   //
   // This protocol definition provides gRPC services for:
   // - Security compliance automation (SCAP, CIS, NIST)
   // - Evidence collection with chain of custody
   // - Asset inventory and configuration management
   // - System monitoring and audit trail
   //
   // Transformed from nexus.proto with all offensive security
   // capabilities removed. See docs/proto/nexus-to-gov-mapping.md
   // for complete transformation documentation.
   //
   // Version: 1.0.0
   // Last Updated: [DATE]
   ```

**Validation:**
- Header clearly states purpose
- References mapping document
- Version information included

**Deliverable:** Documented gov.proto header

---

### Step 10.2: Add Message-Level Documentation
**Goal:** Document each message and enum

**Actions:**
1. Add doc comments to all messages, example:
   ```protobuf
   // ComplianceCheck represents a single compliance control check
   // against a security framework (CIS, NIST, PCI-DSS, etc.)
   message ComplianceCheck {
     // Unique identifier for this check
     string check_id = 1;
     // Standard framework name (e.g., "CIS", "NIST CSF")
     string standard = 2;
     // Control identifier within framework (e.g., "1.1.1", "AC-2")
     string control_id = 3;
     ...
   }
   ```

2. Document all enums similarly

**Validation:**
- All messages have doc comments
- All enums have doc comments
- All fields have inline comments

**Deliverable:** Fully documented gov.proto

---

### Step 10.3: Add RPC Method Documentation
**Goal:** Document service methods with usage examples

**Actions:**
1. Add doc comments to each RPC method:
   ```protobuf
   service GovernanceService {
     // RegisterAgent enrolls a new agent in the governance system.
     // The agent provides system information and receives configuration.
     rpc RegisterAgent(RegistrationRequest) returns (RegistrationResponse);

     // SubmitComplianceCheck executes a single compliance control check
     // and returns the result with evidence references.
     rpc SubmitComplianceCheck(ComplianceCheck) returns (CheckResult);

     ...
   }
   ```

**Validation:**
- All RPC methods documented
- Usage clear from comments
- Request/response types explained

**Deliverable:** Fully documented service definition

---

### Step 10.4: Final Proto Compilation Test
**Goal:** Confirm proto compiles without errors

**Actions:**
1. Compile proto to descriptor set:
   ```bash
   protoc --proto_path=/home/cmndcntrl/code/rust-nexus/nexus-infra/proto \
          --descriptor_set_out=/tmp/gov.desc \
          /home/cmndcntrl/code/rust-nexus/nexus-infra/proto/gov.proto
   ```

2. Inspect descriptor:
   ```bash
   protoc --decode_raw < /tmp/gov.desc | head -100
   ```

**Validation:**
- Compilation succeeds
- Descriptor file created
- No warnings or errors

**Deliverable:** Validated compiled proto

---

### Step 10.5: Generate Language Bindings Test
**Goal:** Verify proto can generate Rust code

**Actions:**
1. Test Rust code generation:
   ```bash
   protoc --proto_path=/home/cmndcntrl/code/rust-nexus/nexus-infra/proto \
          --rust_out=/tmp/proto-test \
          --tonic_out=/tmp/proto-test \
          /home/cmndcntrl/code/rust-nexus/nexus-infra/proto/gov.proto
   ```

2. Verify generated files:
   ```bash
   ls -la /tmp/proto-test
   ```

**Validation:**
- Rust code generated successfully
- gov.v1.rs created
- tonic service code created
- No compilation errors

**Deliverable:** Verified Rust binding generation

---

## Phase 11: Quality Assurance

### Step 11.1: Security Review
**Goal:** Confirm no offensive capabilities remain

**Actions:**
1. Search gov.proto for offensive keywords:
   - shellcode, injection, hollowing
   - credential, harvest, keylog
   - destruct, cleaning, artifact removal
   - BOF, COFF

2. Verify search returns no matches

**Validation:**
- No offensive keywords found
- No attack techniques present
- Only defensive/compliance capabilities

**Deliverable:** Security review sign-off

---

### Step 11.2: Compliance Alignment Review
**Goal:** Verify GRC framework alignment

**Actions:**
1. Review task types against standard frameworks:
   - NIST Cybersecurity Framework
   - CIS Controls
   - ISO 27001
   - PCI-DSS requirements

2. Confirm coverage of common compliance needs

**Validation:**
- Major frameworks addressable
- Common audit tasks supported
- Evidence collection adequate

**Deliverable:** Framework alignment matrix

---

### Step 11.3: Field Number Audit
**Goal:** Ensure no field number conflicts

**Actions:**
1. Extract all field numbers from gov.proto
2. Check for duplicates within each message
3. Verify reserved ranges not used
4. Confirm backward compatibility for retained messages

**Validation:**
- No duplicate field numbers
- Sequential numbering where possible
- Reserved ranges (19000-19999) avoided

**Deliverable:** Field number audit report

---

### Step 11.4: Cross-Reference Documentation
**Goal:** Ensure all documentation is consistent

**Actions:**
1. Verify this implementation plan matches gov.proto
2. Check mapping document accuracy
3. Confirm proto reference documentation complete
4. Validate all file paths correct

**Validation:**
- Documentation matches implementation
- No orphaned references
- All paths absolute and correct

**Deliverable:** Documentation consistency verification

---

## Implementation Checklist

Use this checklist to track progress through the Baby Steps™ plan:

### Phase 1: Proto File Structure Setup
- [ ] Step 1.1: Create base gov.proto file
- [ ] Step 1.2: Copy core infrastructure messages
- [ ] Step 1.3: Copy task management infrastructure
- [ ] Step 1.4: Copy file operations messages
- [ ] Step 1.5: Copy configuration messages

### Phase 2: Remove Offensive Task Types
- [ ] Step 2.1: Document original task types for removal
- [ ] Step 2.2: Create new TaskType enum - core operations

### Phase 3: Add Compliance Task Types
- [ ] Step 3.1: Add SCAP/OVAL compliance tasks
- [ ] Step 3.2: Add registry and file audit tasks
- [ ] Step 3.3: Add service and access review tasks
- [ ] Step 3.4: Add evidence collection tasks
- [ ] Step 3.5: Add asset management tasks

### Phase 4: Create New Compliance Messages
- [ ] Step 4.1: Create ComplianceCheck message
- [ ] Step 4.2: Create CheckResult message
- [ ] Step 4.3: Create ControlAssessment message
- [ ] Step 4.4: Create Evidence collection messages
- [ ] Step 4.5: Create EvidenceChunk message
- [ ] Step 4.6: Create ChainOfCustody message
- [ ] Step 4.7: Create AssetInventory message
- [ ] Step 4.8: Create ConfigurationBaseline message

### Phase 5: Update Artifact Types
- [ ] Step 5.1: Review current ArtifactType enum
- [ ] Step 5.2: Create updated ArtifactType enum

### Phase 6: Remove Offensive RPC Methods
- [ ] Step 6.1: Document RPC methods for removal
- [ ] Step 6.2: Create GovernanceService RPC definition

### Phase 7: Add New Compliance RPC Methods
- [ ] Step 7.1: Add compliance check RPC methods
- [ ] Step 7.2: Add evidence collection RPC methods
- [ ] Step 7.3: Add asset management RPC methods

### Phase 8: Validation and Documentation
- [ ] Step 8.1: Complete proto validation
- [ ] Step 8.2: Generate proto documentation
- [ ] Step 8.3: Create migration mapping document
- [ ] Step 8.4: Verify compatibility

### Phase 9: Integration Points
- [ ] Step 9.1: Identify Rust code dependencies
- [ ] Step 9.2: Identify build configuration dependencies
- [ ] Step 9.3: Document required Rust changes

### Phase 10: Proto File Finalization
- [ ] Step 10.1: Add proto file header comments
- [ ] Step 10.2: Add message-level documentation
- [ ] Step 10.3: Add RPC method documentation
- [ ] Step 10.4: Final proto compilation test
- [ ] Step 10.5: Generate language bindings test

### Phase 11: Quality Assurance
- [ ] Step 11.1: Security review
- [ ] Step 11.2: Compliance alignment review
- [ ] Step 11.3: Field number audit
- [ ] Step 11.4: Cross-reference documentation

---

## Success Criteria

The proto transformation is complete when:

1. gov.proto contains zero offensive security capabilities
2. All compliance task types implemented
3. All new compliance messages defined
4. GovernanceService RPC methods complete
5. Proto compiles without errors
6. Rust bindings generate successfully
7. Documentation complete and accurate
8. Security review passed
9. Backward compatibility verified for retained messages
10. All integration points documented

---

## Next Phase

After completing this proto transformation, proceed to:
- Phase 10: Rust Code Transformation (update imports, service implementations)
- Phase 20: Test Suite Transformation (update proto message tests)
- Phase 30: Integration Testing (verify end-to-end compliance workflows)

---

## Notes

**Remember the Baby Steps™ methodology:**
- Complete each step fully before moving to the next
- Validate after every change
- Document as you go (the process is the product)
- One substantive accomplishment at a time
- Build incrementally, test continuously

This transformation removes all offensive security capabilities while providing comprehensive governance, risk, and compliance functionality suitable for enterprise security automation and audit purposes.
