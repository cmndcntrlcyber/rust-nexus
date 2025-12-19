# 📊 Before/After Architecture Diagrams

> Visual comparison of the transformation.

## 🏗️ Overall System Architecture

### Before (C2 Framework)

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│    Operator     │     │   C2 Server     │     │     Agents      │
│       UI        │────►│    (gRPC)       │────►│   (Targets)     │
└─────────────────┘     └─────────────────┘     └─────────────────┘
                               │
                               ▼
                        ┌─────────────────┐
                        │  Infrastructure │
                        │ • DNS rotation  │
                        │ • Certificates  │
                        │ • Domain front  │
                        └─────────────────┘
```

### After (SOC Platform)

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  SOC Dashboard  │     │   Detection     │     │   EDR Agents    │
│    (WASM UI)    │────►│    Server       │────►│  (Endpoints)    │
└─────────────────┘     └─────────────────┘     └─────────────────┘
        │                       │                       │
        ▼                       ▼                       ▼
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│      SIEM       │◄────│   LitterBox     │◄────│   Detection     │
│   Connectors    │     │   Analysis      │     │    Events       │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

## 🔄 Data Flow Comparison

### Before: C2 Task Flow
<!-- TODO: Add C2 data flow -->

### After: Detection Flow
<!-- TODO: Add detection data flow -->

## 🧩 Component Evolution

<!-- TODO: Add component diagrams -->

---
**Version**: 0.1.0 (scaffold)
**Last Updated**: 2024-12-19
**Maintained By**: Architecture Lead
