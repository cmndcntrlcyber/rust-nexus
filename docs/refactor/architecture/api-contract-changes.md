# 📡 API Contract Changes

> gRPC and API migration guide.

## 📋 Overview

<!-- TODO: Add API change overview -->

This document tracks changes to the gRPC protocol and other APIs during transformation.

## 🔧 Proto File Changes

**Location**: `nexus-infra/proto/nexus.proto`

### Service Changes

| Original Service | Original Purpose | New Service | New Purpose |
|------------------|------------------|-------------|-------------|
| `NexusC2.RegisterAgent` | Register C2 agent | `NexusSOC.RegisterAgent` | Register EDR agent |
| `NexusC2.GetTasks` | Get offensive tasks | `NexusSOC.GetDetectionTasks` | Get detection tasks |
| `NexusC2.ExecuteShellcode` | Execute payload | `NexusSOC.ExecuteResponse` | Execute remediation |

### Message Changes

<!-- TODO: Document message changes -->

## 🔄 Backward Compatibility

<!-- TODO: Document compatibility approach -->

### Migration Strategy
1. Add new services alongside old
2. Deprecate old services
3. Remove old services after migration

## 📝 Breaking Changes

<!-- TODO: Document breaking changes -->

---
**Version**: 0.1.0 (scaffold)
**Last Updated**: 2024-12-19
**Maintained By**: Architecture Lead
