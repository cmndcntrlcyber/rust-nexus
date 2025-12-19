# 🔗 Dependency Matrix

> Cross-agent and cross-phase dependencies.

## 📋 Overview

This document tracks what each component depends on and what it blocks.

## 🏗️ Phase 1 Dependencies

### Component Dependencies

```
nexus-detection (crate)
├── depends on: nexus-common (types, encryption)
├── depends on: nexus-infra (for LitterBox integration)
└── provides to: Phase 2 (detection APIs)

LitterBox Deployment
├── depends on: nexus-infra/domain_manager.rs
├── depends on: nexus-infra/cert_manager.rs
└── provides to: nexus-detection (analysis API)

Event Pipeline
├── depends on: nexus-detection/signature
├── depends on: nexus-detection/behavioral
└── provides to: Phase 3 (alert data)
```

### Agent Dependencies

| Agent | Waits For | Blocks |
|-------|-----------|--------|
| Detection Engine | None | SOC Platform, Testing |
| Infrastructure | None | Detection Engine (LitterBox) |
| SOC Platform | Detection Engine, Infrastructure | Integration |
| Integration | SOC Platform | None |
| Testing | All (for validation) | None |

## 🔄 Phase 2 Dependencies

<!-- TODO: Add Phase 2 dependencies -->

```
gRPC Transformation
├── depends on: Phase 1 complete
└── provides to: Agent transformation

Agent EDR Mode
├── depends on: gRPC transformation
└── provides to: SOC Platform
```

## 🔀 Cross-Phase Dependencies

<!-- TODO: Add cross-phase diagram -->

```
Phase 1 ──► Phase 2 ──► Phase 3 ──► Phase 4
   │           │           │           │
   └─────────────────────────────────►─┘
         (nexus-detection used in all)
```

## ⚠️ Critical Path

<!-- TODO: Identify bottleneck tasks -->

The following tasks are on the critical path:

1. DET-001: nexus-detection scaffold
2. DET-002: Signature engine
3. INF-001: LitterBox deployment
4. DET-003: Event pipeline

---
**Version**: 0.1.0 (scaffold)
**Last Updated**: 2024-12-19
**Maintained By**: Architecture Lead
