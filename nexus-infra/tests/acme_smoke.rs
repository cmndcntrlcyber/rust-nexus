//! v1.4.1 ACME smoke test (Phase 1.4.1 / D-V1.4-A).
//!
//! `staging_dns01_round_trip` activates when these env vars are all set:
//!
//! - `LETSENCRYPT_STAGING_ENABLED=1` (gate)
//! - `LETSENCRYPT_TEST_DOMAIN` — primary domain to issue against
//! - `CLOUDFLARE_API_TOKEN` — token scoped to `Zone:DNS:Edit` for the
//!   zone serving `LETSENCRYPT_TEST_DOMAIN`
//! - `CLOUDFLARE_ZONE_ID` — the zone id (Cloudflare dashboard →
//!   Overview → Zone ID)
//! - Optional: `LETSENCRYPT_CONTACT_EMAIL` (default
//!   `ops@example.com`)
//!
//! Dials Let's Encrypt staging, provisions a real cert via DNS-01,
//! and asserts the saved cert + key files exist.
//!
//! `#[ignore]`d by default so CI without staging access skips cleanly.
//! This test makes real network calls when activated.

use std::path::PathBuf;

use nexus_infra::cloudflare::CloudflareManager;
use nexus_infra::letsencrypt::CertificateManager;

#[tokio::test]
#[ignore = "needs LETSENCRYPT_STAGING_ENABLED=1 + Cloudflare creds + a real domain"]
async fn staging_dns01_round_trip() {
    let staging_enabled = std::env::var("LETSENCRYPT_STAGING_ENABLED").unwrap_or_default() == "1";
    assert!(staging_enabled, "guard env var");

    let domain = std::env::var("LETSENCRYPT_TEST_DOMAIN")
        .expect("LETSENCRYPT_TEST_DOMAIN required for staging test");
    let api_token = std::env::var("CLOUDFLARE_API_TOKEN")
        .expect("CLOUDFLARE_API_TOKEN required for staging test");
    let zone_id =
        std::env::var("CLOUDFLARE_ZONE_ID").expect("CLOUDFLARE_ZONE_ID required for staging test");
    let contact_email = std::env::var("LETSENCRYPT_CONTACT_EMAIL")
        .unwrap_or_else(|_| "ops@example.com".to_string());

    let tmp = tempfile::tempdir().expect("tempdir");

    let cf_config = nexus_infra::config::CloudflareConfig {
        api_token: api_token.into(),
        zone_id,
        domain: domain.clone(),
        ..nexus_infra::config::CloudflareConfig::default()
    };
    let cloudflare = CloudflareManager::new(cf_config).expect("cf manager");

    let le_config = nexus_infra::config::LetsEncryptConfig {
        acme_directory_url: "https://acme-staging-v02.api.letsencrypt.org/directory".to_string(),
        contact_email,
        cert_storage_dir: tmp.path().to_path_buf(),
        cert_renewal_days: 30,
        ..Default::default()
    };
    let mut manager = CertificateManager::new(le_config, cloudflare);
    manager.initialize().await.expect("initialize");

    let cert_info = manager
        .request_certificate(&domain, &[])
        .await
        .expect("request_certificate");

    assert!(cert_info.cert_path.exists(), "cert file written");
    assert!(cert_info.key_path.exists(), "key file written");
    assert!(cert_info.expires_at > chrono::Utc::now());
}

// Pure-Rust check that doesn't dial staging: builds the manager,
// confirms `initialize` creates the cert storage dir. Always runs.
#[tokio::test]
async fn initialize_creates_storage_dir() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let storage = tmp.path().join("certs");
    assert!(!storage.exists(), "fresh dir doesn't exist yet");

    let cf_config = nexus_infra::config::CloudflareConfig::default();
    let cloudflare = CloudflareManager::new(cf_config).expect("cf manager");
    let le_config = nexus_infra::config::LetsEncryptConfig {
        acme_directory_url: "https://acme-staging-v02.api.letsencrypt.org/directory".to_string(),
        contact_email: "ops@example.com".to_string(),
        cert_storage_dir: storage.clone(),
        cert_renewal_days: 30,
        ..Default::default()
    };
    let mut manager = CertificateManager::new(le_config, cloudflare);
    manager.initialize().await.expect("initialize");
    assert!(storage.exists(), "initialize creates the storage dir");
    let _ = PathBuf::from("/tmp"); // silence unused-import warning if any
}
