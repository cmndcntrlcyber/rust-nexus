# (Moved)

The Cloudflare DNS + ACME pipeline is **optional in v1.2** and partially
stubbed. See the [Cloudflare/ACME appendix in the production deployment
doc](../deployment/production.md#cloudflareacme-appendix) for current
status.

The overlay-era setup instructions (API token provisioning, zone config,
domain rotation tuning) are preserved in git history at the path
`docs/infrastructure/cloudflare-setup.md@<pre-v1.2-commit>`.

For v1.2 production rollouts, follow
[`../deployment/production.md`](../deployment/production.md) end-to-end;
the Cloudflare integration only needs to be re-enabled if you want
domain-fronting / DNS rotation features. Certificate provisioning is
fully external in v1.2.
