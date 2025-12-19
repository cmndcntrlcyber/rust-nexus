# 🤖 Agent Roles

> Specialized agent definitions for the transformation project.

## 📋 Overview

Five specialized agent roles work together on the transformation:

| Role | Focus | Primary Phases |
|------|-------|----------------|
| Detection Engine | nexus-detection crate | 1, 2 |
| Infrastructure | LitterBox, DNS automation | 1 |
| SOC Platform | Dashboard, response | 3 |
| Integration | SIEM connectors | 4 |
| Testing | Validation, QA | All |

---

## 🔍 Detection Engine Agent

**Agent ID**: `detection-engine`

### Responsibilities
- Implement nexus-detection crate
- Port signature engine from reverse-shell-detector
- Build behavioral analysis capabilities
- Create process-network correlation

### Required Knowledge
- Rust async/await patterns
- Network packet analysis
- Process monitoring
- Signature matching algorithms

### Phase Assignments
- **Phase 1**: Primary on nexus-detection, event pipeline
- **Phase 2**: Primary on agent transformation, detection capabilities
- **Phase 3**: Support for detection integration

### Key Files
<!-- TODO: Add key file paths -->

---

## 🏗️ Infrastructure Agent

**Agent ID**: `infrastructure`

### Responsibilities
- Automate LitterBox deployment
- Adapt DNS/cert infrastructure
- Build geographic distribution
- Maintain existing nexus-infra

### Required Knowledge
- nexus-infra crate internals
- Docker/containerization
- Cloudflare APIs
- TLS/certificate management

### Phase Assignments
- **Phase 1**: Primary on LitterBox deployment
- **Phase 2**: Support for gRPC transformation

### Key Files
- `nexus-infra/src/domain_manager.rs`
- `nexus-infra/src/cert_manager.rs`
<!-- TODO: Add more key files -->

---

## 🖥️ SOC Platform Agent

**Agent ID**: `soc-platform`

### Responsibilities
- Build SOC management dashboard
- Implement asset monitoring
- Create response orchestration
- Design workflow engine

### Required Knowledge
- Web UI development (WASM preferred)
- API design
- Dashboard/visualization
- Workflow automation

### Phase Assignments
- **Phase 3**: Primary on all components
- **Phase 4**: Support for integration UI

### Key Files
<!-- TODO: Add key file paths -->

---

## 🔗 Integration Agent

**Agent ID**: `integration`

### Responsibilities
- Build SIEM connectors (Splunk, QRadar, Sentinel)
- Integrate threat intelligence feeds
- Implement compliance reporting
- Create external API interfaces

### Required Knowledge
- SIEM APIs and protocols
- Threat intel standards (STIX/TAXII)
- Compliance frameworks
- API integration patterns

### Phase Assignments
- **Phase 4**: Primary on all components

### Key Files
<!-- TODO: Add key file paths -->

---

## 🧪 Testing Agent

**Agent ID**: `testing`

### Responsibilities
- Validate detection accuracy
- Perform integration testing
- Run performance benchmarks
- Verify phase completion

### Required Knowledge
- Rust testing frameworks
- Integration test design
- Performance profiling
- Security testing

### Phase Assignments
- **All Phases**: Validation support

### Key Files
- `docs/refactor/testing/`
<!-- TODO: Add test file paths -->

---
**Version**: 0.1.0 (scaffold)
**Last Updated**: 2024-12-19
**Maintained By**: Architecture Lead
