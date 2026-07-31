# Phase 1: Healthcheck Grade Remediation (v1.5)

**Goal:** Bring every healthcheck area to grade **A**, working worst-first.
**Baseline:** `docs/enhancements/healthcheck.md` dated 2026-06-15.

Each area file defines its **A threshold**, then breaks the work into
numbered stages of individually-completable baby steps. Every step names
the exact file(s) and the exact change. Every stage ends with a
verification command.

---

## Area Plans

| File | Area | Grade | Target | Stages | Steps |
|------|------|-------|--------|--------|-------|
| [v1.5.1](v1.5.1.md) | Build Health | **F** | A | 1.1–1.4 | 5 |
| [v1.5.2](v1.5.2.md) | Security Hygiene | **C** | A | 2.1–2.4 | 9 |
| [v1.5.3](v1.5.3.md) | Test Coverage | **C+** | A | 3.1–3.6 | 12 |
| [v1.5.4](v1.5.4.md) | Observability | **B-** | A | 4.1–4.5 | 16 |
| [v1.5.5](v1.5.5.md) | Dependency Mgmt | **B** | A | 5.1–5.3 | 5 |
| [v1.5.6](v1.5.6.md) | Documentation | **B** | A | 6.1–6.4 | 10 |
| [v1.5.7](v1.5.7.md) | Code Quality | **B+** | A | 7.1–7.4 | 12 |
| [v1.5.8](v1.5.8.md) | CI/CD | **A-** | A | 8.1–8.4 | 5 |
| [v1.5.9](v1.5.9.md) | Infra Reliability | **B** | A | 9.1–9.11 | 29 |

**Total: 44 stages, ~103 steps**

---

## Critical Dependencies

- **v1.5.3** Stage 3.1 and **v1.5.8** Stage 8.1 depend on **v1.5.1** Stage 1.1 (build fix).
- **v1.5.4** Stage 4.4 depends on Stages 4.1–4.3 (all crates migrated off `log`).
- **v1.5.5** Stage 5.1 overlaps **v1.5.1** Stage 1.3 (hickory-dns removal).
- All other stages are independent and can be parallelized.

---

## Final Verification

After all area plans are complete, re-run the full healthcheck audit:

```bash
cargo check --workspace --exclude nexus-console --all-targets
cargo test --workspace --exclude nexus-console --all-targets
cargo clippy --workspace --exclude nexus-console --all-targets -- -D warnings
RUSTDOCFLAGS="-D missing_docs" cargo doc -p nexus-common --no-deps
grep -rn "use log::" --include="*.rs" nexus-*/src/
grep -rn "#\!\[allow(dead_code)\]" --include="*.rs" nexus-*/src/
grep -rn "// TODO:" --include="*.rs" nexus-*/src/ | grep -v "TODO(#\|DEPRECATED"
```

All commands should produce zero errors/warnings/matches. Every healthcheck
area should now grade **A**.
