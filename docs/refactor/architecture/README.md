# 🏗️ Architecture Documentation

> Architectural reference for the C2→SOC transformation.

## 📋 Quick Navigation

| Document | Description |
|----------|-------------|
| [before-after-diagrams.md](before-after-diagrams.md) | Visual transformation comparison |
| [component-mapping.md](component-mapping.md) | C2 component → SOC component mapping |
| [data-flow-transformation.md](data-flow-transformation.md) | How data flows change |
| [api-contract-changes.md](api-contract-changes.md) | gRPC/API migration guide |

## 🎯 Transformation Summary

```
BEFORE (C2)                           AFTER (SOC)
─────────────────────────────────────────────────────────
nexus-agent (offensive)    ────►   EDR Agent (detection)
nexus-infra (C2 server)    ────►   Detection Server
evasion.rs                 ────►   evasion_detection.rs
execution.rs               ────►   response_execution.rs
domain rotation            ────►   Infrastructure mgmt
```

## 📊 High-Level Architecture

### Before
<!-- TODO: Add before diagram -->

### After
<!-- TODO: Add after diagram -->

## 🔧 Key Decisions

<!-- TODO: Document architectural decisions -->

---
**Version**: 0.1.0 (scaffold)
**Last Updated**: 2024-12-19
**Maintained By**: Architecture Lead
