# 📞 Communication Protocol

> How agents coordinate during transformation work.

## 📋 Overview

<!-- TODO: Add communication philosophy -->

This document defines how development agents should communicate and coordinate during the transformation.

## 🔄 Sync Mechanisms

### Daily Status Updates

<!-- TODO: Define update process -->

Agents update [Task-Distribution.md](Task-Distribution.md) with:
- Completed tasks
- Current work
- Blockers
- Next steps

### Blocker Escalation

<!-- TODO: Define escalation process -->

When blocked:
1. Document in Task-Distribution.md
2. Tag blocking agent
3. Propose resolution if possible
4. Escalate to Architecture Lead if unresolved

## 📝 Documentation Updates

### When to Update

- Completing a baby step
- Discovering new dependencies
- Changing architecture
- Finding issues

### What to Update

- Relevant phase docs
- Dependency-Matrix.md
- Progress-Dashboard.md
- Code comments

## 🔀 Merge Coordination

<!-- TODO: Define merge process -->

### Shared Files

Files touched by multiple agents:
- `Cargo.toml` (workspace)
- `nexus.toml` (configuration)
- Proto files

### Conflict Resolution

1. Check Dependency-Matrix.md for ownership
2. Coordinate with affected agent
3. Use feature flags if parallel work needed
4. Document resolution

## ⚠️ Critical Alerts

<!-- TODO: Define alert triggers -->

Immediately communicate:
- Breaking changes to shared APIs
- New dependencies discovered
- Blockers affecting critical path
- Security concerns

---
**Version**: 0.1.0 (scaffold)
**Last Updated**: 2024-12-19
**Maintained By**: Architecture Lead
