# (Moved)

v1.2 certificate provisioning is documented at
[`../deployment/production.md#ca-strategy`](../deployment/production.md#ca-strategy).

For local-dev cert provisioning via `scripts/gen-certs.sh`, see
[`../deployment/local-dev.md#4-provision-dev-mtls-certs`](../deployment/local-dev.md#4-provision-dev-mtls-certs).

The original overlay-era ACME / origin-cert content is preserved in
git history. v1.2.1 stubbed the in-process ACME workflow — production
deployments provide their own CA. The full ACME re-port is v1.3 work
(see `nexus-infra/src/letsencrypt.rs`).
