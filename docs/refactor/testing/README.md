# 🧪 Testing Strategy

> Testing and validation approach for the transformation.

## 📋 Quick Navigation

| Document | Description |
|----------|-------------|
| [unit-testing-guide.md](unit-testing-guide.md) | Unit test requirements |
| [integration-testing.md](integration-testing.md) | Cross-component testing |
| [detection-validation.md](detection-validation.md) | Validating detection accuracy |
| [validation-checklists/](validation-checklists/) | Phase completion checklists |

## 🔺 Testing Pyramid

```
           ┌─────────┐
           │   E2E   │  ← SOC workflow tests
          ┌┴─────────┴┐
          │Integration│  ← Cross-crate tests
         ┌┴───────────┴┐
         │    Unit     │  ← Per-module tests
        └──────────────┘
```

## 📊 Coverage Requirements

| Level | Requirement |
|-------|-------------|
| Unit tests | 80% minimum per module |
| Integration | All API boundaries |
| E2E | Critical SOC workflows |

## 🔧 Running Tests

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p nexus-detection

# Integration tests
cargo test --test integration_*
```

## ✅ Validation Checklists

- [Phase 1 Validation](validation-checklists/phase-1-validation.md)
- [Phase 2 Validation](validation-checklists/phase-2-validation.md)
- [Phase 3 Validation](validation-checklists/phase-3-validation.md)
- [Phase 4 Validation](validation-checklists/phase-4-validation.md)

---
**Version**: 0.1.0 (scaffold)
**Last Updated**: 2024-12-19
**Maintained By**: Testing Agent
