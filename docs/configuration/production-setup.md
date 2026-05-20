# (Moved)

This document has moved to
[`../deployment/production.md`](../deployment/production.md) as of v1.2
(2026-05-19). The original overlay-era content is preserved in git
history.

**Why moved.** v1.2 deployments use operator-provided CAs and the new
`NEXUS_*_CERT` env vars rather than the original Cloudflare + ACME
pipeline (which is partially deferred to v1.3 — see
`nexus-infra/src/letsencrypt.rs`).

For the legacy Cloudflare/ACME pipeline notes, see
[`../deployment/production.md#cloudflareacme-appendix`](../deployment/production.md#cloudflareacme-appendix).
