# 🎯 Phase 1: Integration

> Create detection capabilities and LitterBox integration using existing infrastructure.

## 📋 Overview

**Duration**: 4-6 weeks
**Status**: ⏳ Pending
**Dependencies**: None (first phase)

Phase 1 establishes the foundation for detection capabilities by:
- Creating the `nexus-detection` crate
- Automating LitterBox sandbox deployment
- Building event correlation pipeline

## 🏗️ Architecture

<!-- TODO: Add phase 1 architecture diagram -->

```
┌─────────────────┐     ┌─────────────────┐
│ nexus-detection │────►│   LitterBox     │
│                 │     │   (Docker)      │
├─────────────────┤     └─────────────────┘
│ • Signatures    │              │
│ • Behavioral    │              ▼
│ • Network       │     ┌─────────────────┐
│ • Process       │◄────│ Analysis Results│
└─────────────────┘     └─────────────────┘
         │
         ▼
┌─────────────────┐
│ Event Pipeline  │
└─────────────────┘
```

## 📚 Documentation

| Document | Description |
|----------|-------------|
| [nexus-detection-crate.md](nexus-detection-crate.md) | New crate implementation guide |
| [litterbox-deployment.md](litterbox-deployment.md) | LitterBox automated deployment |
| [event-correlation.md](event-correlation.md) | Event processing design |

## 🐾 Baby Steps

| Step | Task | Status |
|------|------|--------|
| 1 | [Detection Scaffold](baby-steps/01-detection-scaffold.md) | ⏳ Pending |
| 2 | [Signature Engine](baby-steps/02-signature-engine.md) | ⏳ Pending |
| 3 | [LitterBox API](baby-steps/03-litterbox-api.md) | ⏳ Pending |
| 4 | [Event Pipeline](baby-steps/04-event-pipeline.md) | ⏳ Pending |

See [completion-checklist.md](baby-steps/completion-checklist.md) for validation.

## 🤖 Agent Assignments

<!-- TODO: Assign agents to components -->

| Component | Primary Agent | Support |
|-----------|---------------|---------|
| nexus-detection | Detection Engine | - |
| LitterBox deploy | Infrastructure | Detection Engine |
| Event pipeline | Detection Engine | Infrastructure |

## ✅ Success Criteria

- [ ] `cargo build -p nexus-detection` succeeds
- [ ] `cargo test -p nexus-detection` passes
- [ ] LitterBox deploys via existing DomainManager/CertManager
- [ ] Events flow from detection to correlation pipeline
- [ ] Integration tests pass

## 🔗 Dependencies

**Uses from existing crates**:
- `nexus-common`: Types, encryption
- `nexus-infra`: DomainManager, CertManager, gRPC

**External**:
- [reverse-shell-detector](https://github.com/example/reverse-shell-detector) signatures
- [LitterBox](https://github.com/BlackSnufkin/LitterBox) sandbox

---
**Version**: 0.1.0 (scaffold)
**Last Updated**: 2024-12-19
**Maintained By**: Phase 1 Lead
