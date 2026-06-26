//! Programmatic PKI: all certs issued by Cloudflare.
//!
//! **CF mode** (`cf_token` + `cf_zone_id` + `cf_client_ca`):
//! - Server cert: CSR sent to CF Origin CA API → 15-year cert signed by CF
//! - Agent / operator / console client certs: CSR sent to CF Client Certs API
//!   → cert signed by CF zone-managed CA
//! - `ca.crt.pem`: copied from the CF zone CA cert supplied via `cf_client_ca`
//!   (downloaded once from the CF dashboard; no private key needed or written)
//! - `server-ca.crt.pem`: embedded [`CF_ORIGIN_CA_ECC_ROOT`] constant
//!
//! **Self-signed mode** (`cf_token` / `cf_zone_id` = `None`):
//! - Internal CA generated with `rcgen`; all certs signed locally
//! - `ca.crt.pem` + `ca.key.pem`: internal CA (key needed for `add_agent`)
//!
//! Private keys are always generated locally and never sent to Cloudflare.

// Some PKI helpers are only called in the CLI path (nexus-server pki).

use std::net::IpAddr;
use std::path::{Path, PathBuf};

use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DistinguishedName, DnType,
    ExtendedKeyUsagePurpose, IsCa, KeyPair, SanType, PKCS_ECDSA_P256_SHA256,
};

use crate::{InfraError, InfraResult};

// ---------------------------------------------------------------------------
// CF Origin CA ECC root (embedded; expires 2029-08-15).
// Source: https://developers.cloudflare.com/ssl/static/origin_ca_ecc_root.pem
// ---------------------------------------------------------------------------

/// Cloudflare Origin CA ECC root certificate (PEM).
///
/// Set `NEXUS_CA_CERT` to this on agent / console / operator so they can
/// verify the server cert issued by CF Origin CA.
pub const CF_ORIGIN_CA_ECC_ROOT: &str = "\
-----BEGIN CERTIFICATE-----\n\
MIICiTCCAi6gAwIBAgIUXZP3MWb8MKwBE1Qbawsp1sfA/Y4wCgYIKoZIzj0EAwIw\n\
gY8xCzAJBgNVBAYTAlVTMRMwEQYDVQQIEwpDYWxpZm9ybmlhMRYwFAYDVQQHEw1T\n\
YW4gRnJhbmNpc2NvMRkwFwYDVQQKExBDbG91ZEZsYXJlLCBJbmMuMTgwNgYDVQQL\n\
Ey9DbG91ZEZsYXJlIE9yaWdpbiBTU0wgRUNDIENlcnRpZmljYXRlIEF1dGhvcml0\n\
eTAeFw0xOTA4MjMyMTA4MDBaFw0yOTA4MTUxNzAwMDBaMIGPMQswCQYDVQQGEwJV\n\
UzETMBEGA1UECBMKQ2FsaWZvcm5pYTEWMBQGA1UEBxMNU2FuIEZyYW5jaXNjbzEZ\n\
MBcGA1UEChMQQ2xvdWRGbGFyZSwgSW5jLjE4MDYGA1UECxMvQ2xvdWRGbGFyZSBP\n\
cmlnaW4gU1NMIEVDQyBDZXJ0aWZpY2F0ZSBBdXRob3JpdHkwWTATBgcqhkjOPQIB\n\
BggqhkjOPQMBBwNCAASR+sGALuaGshnUbcxKry+0LEXZ4NY6JUAtSeA6g87K3jaA\n\
xpIg9G50PokpfWkhbarLfpcZu0UAoYy2su0EhN7wo2YwZDAOBgNVHQ8BAf8EBAMC\n\
AQYwEgYDVR0TAQH/BAgwBgEB/wIBAjAdBgNVHQ4EFgQUhTBdOypw1O3VkmcH/es5\n\
tBoOOKcwHwYDVR0jBBgwFoAUhTBdOypw1O3VkmcH/es5tBoOOKcwCgYIKoZIzj0E\n\
AwIDSQAwRgIhAKilfntP2ILGZjwajktkBtXE1pB4Y/fjAfLkIRUzrI15AiEA5UCL\n\
XYZZ9m2c3fKwIenMMojL1eqydsgqj/wK4p5kagQ=\n\
-----END CERTIFICATE-----\n";

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Paths written by [`PkiManager::init`].
#[derive(Debug, Clone)]
pub struct PkiBundle {
    pub out_dir: PathBuf,

    /// CA cert agents/console/operator use to verify the **server** cert.
    /// Always `server-ca.crt.pem` (CF Origin CA ECC root in CF mode;
    /// internal CA in self-signed mode).
    pub server_ca_cert: PathBuf,

    /// CA cert the **server** uses to verify incoming client certs.
    /// Always `ca.crt.pem` (CF zone CA cert in CF mode; internal CA in
    /// self-signed mode). No corresponding key file in CF mode.
    pub client_ca_cert: PathBuf,

    pub server_cert: PathBuf,
    pub server_key: PathBuf,

    pub console_cert: PathBuf,
    pub console_key: PathBuf,

    pub operator_cert: PathBuf,
    pub operator_key: PathBuf,

    /// One entry per agent, in order.
    pub agents: Vec<AgentBundle>,
}

/// Cert + key paths for a single agent.
#[derive(Debug, Clone)]
pub struct AgentBundle {
    pub name: String,
    pub cert: PathBuf,
    pub key: PathBuf,
}

// ---------------------------------------------------------------------------
// PkiManager
// ---------------------------------------------------------------------------

pub struct PkiManager;

impl PkiManager {
    /// Generate a complete PKI in one atomic operation.
    ///
    /// **CF mode** — all three CF parameters must be `Some`:
    /// - `cf_token`: API token with `Zone > SSL+Certs > Edit` AND
    ///   `Zone > Client Certificates > Edit`
    /// - `cf_zone_id`: your Cloudflare zone ID
    /// - `cf_client_ca`: path to the CF zone managed CA cert PEM
    ///   (download from CF dashboard → SSL/TLS → Client Certificates →
    ///   Certificate Authorities)
    ///
    /// **Self-signed mode** — pass `None` for all three CF parameters.
    ///
    /// Returns `Err` if `out_dir` already contains `server.key.pem`.
    pub fn init(
        domain: &str,
        ip: IpAddr,
        agent_count: u8,
        out_dir: &Path,
        cf_token: Option<&str>,
        cf_zone_id: Option<&str>,
        cf_client_ca: Option<&Path>,
    ) -> InfraResult<PkiBundle> {
        // Guard: CF zone_id requires CF client CA cert.
        if cf_zone_id.is_some() && cf_client_ca.is_none() {
            return Err(InfraError::CertificateError(
                "--cf-zone-id requires --cf-client-ca (download CF zone CA cert from \
                 CF dashboard → SSL/TLS → Client Certificates → Certificate Authorities)"
                    .to_string(),
            ));
        }

        let srv_key_path = out_dir.join("server.key.pem");
        if srv_key_path.exists() {
            return Err(InfraError::CertificateError(format!(
                "PKI already initialized at {}; use `pki agent` to mint additional agents",
                out_dir.display()
            )));
        }

        let use_cf_clients = cf_zone_id.is_some();

        // -- Server cert (always: key generated locally, CF Origin CA signs it if token present).
        let server_tmpl = generate_server_cert_template(domain, ip)?;
        let server_key_pem = server_tmpl.serialize_private_key_pem();
        let (server_cert_pem, server_ca_label) = match cf_token {
            Some(token) => {
                let csr = server_tmpl.serialize_request_pem().map_err(|e| {
                    InfraError::CertificateError(format!("server CSR: {e}"))
                })?;
                let cert = call_cf_origin_ca_api(token, &csr, &[domain.to_string()])?;
                (cert, "server-ca")
            }
            None => {
                // Self-signed path: use internal CA (generated below).
                (String::new(), "ca") // placeholder; filled after CA is built
            }
        };

        // -- Internal CA (always built for self-signed mode; skipped for client certs in CF mode).
        let (internal_ca, internal_ca_crt_pem, internal_ca_key_pem) = if !use_cf_clients || cf_token.is_none() {
            let ca = generate_ca("Nexus C2 CA")?;
            let crt = ca.serialize_pem().map_err(|e| {
                InfraError::CertificateError(format!("CA serialize_pem: {e}"))
            })?;
            let key = ca.serialize_private_key_pem();
            (Some(ca), crt, key)
        } else {
            (None, String::new(), String::new())
        };

        // Resolve server cert for self-signed path.
        let (server_cert_pem, server_ca_label) = if cf_token.is_some() {
            (server_cert_pem, server_ca_label)
        } else {
            let ca = internal_ca.as_ref().unwrap();
            let cert = server_tmpl.serialize_pem_with_signer(ca).map_err(|e| {
                InfraError::CertificateError(format!("server serialize_pem: {e}"))
            })?;
            (cert, "ca")
        };

        // -- Client certs (console, operator, agents).
        let console_tmpl = generate_client_cert_template("console")?;
        let operator_tmpl = generate_client_cert_template("operator")?;
        let mut agent_tmpls: Vec<(String, Certificate)> = Vec::new();
        for i in 1..=agent_count {
            let name = format!("agent-{:03}", i);
            agent_tmpls.push((name.clone(), generate_client_cert_template(&name)?));
        }

        // Helper: sign one client cert template.
        let sign_client = |tmpl: &Certificate, cn: &str, token: Option<&str>, zone: Option<&str>|
            -> InfraResult<String>
        {
            if let (Some(tok), Some(zid)) = (token, zone) {
                let csr = tmpl.serialize_request_pem().map_err(|e| {
                    InfraError::CertificateError(format!("{cn} CSR: {e}"))
                })?;
                call_cf_client_cert_api(tok, zid, &csr, 3650)
            } else {
                tmpl.serialize_pem_with_signer(internal_ca.as_ref().unwrap())
                    .map_err(|e| InfraError::CertificateError(format!("{cn} sign: {e}")))
            }
        };

        let console_cert_pem = sign_client(&console_tmpl, "console", cf_token, cf_zone_id)?;
        let console_key_pem = console_tmpl.serialize_private_key_pem();

        let operator_cert_pem = sign_client(&operator_tmpl, "operator", cf_token, cf_zone_id)?;
        let operator_key_pem = operator_tmpl.serialize_private_key_pem();

        let mut agent_certs: Vec<(String, String, String)> = Vec::new();
        for (name, tmpl) in &agent_tmpls {
            let cert = sign_client(tmpl, name, cf_token, cf_zone_id)?;
            let key = tmpl.serialize_private_key_pem();
            agent_certs.push((name.clone(), cert, key));
        }

        // -- Atomic write.
        let tmp_dir = tempfile_dir(out_dir)?;
        let tmp = tmp_dir.as_path();

        // Server CA cert (what clients use to verify server).
        if cf_token.is_some() {
            write_cert(tmp, "server-ca", CF_ORIGIN_CA_ECC_ROOT)?;
        } else {
            // In self-signed mode, server-ca == internal CA.
            write_cert(tmp, "server-ca", &internal_ca_crt_pem)?;
        }

        // Client CA cert (what server uses to verify client certs).
        if use_cf_clients {
            // Copy CF zone CA cert from the path the user provided.
            let cf_ca_pem = std::fs::read_to_string(cf_client_ca.unwrap()).map_err(|e| {
                InfraError::CertificateError(format!("read --cf-client-ca: {e}"))
            })?;
            write_cert(tmp, "ca", &cf_ca_pem)?;
        } else {
            // Self-signed: internal CA serves as both server CA and client CA.
            write_cert(tmp, "ca", &internal_ca_crt_pem)?;
            write_key(tmp, "ca", &internal_ca_key_pem)?;
        }

        // Server cert + key.
        write_cert(tmp, "server", &server_cert_pem)?;
        write_key(tmp, "server", &server_key_pem)?;

        // Console (subdirectory).
        std::fs::create_dir_all(tmp.join("console"))?;
        write_cert(&tmp.join("console"), "client", &console_cert_pem)?;
        write_key(&tmp.join("console"), "client", &console_key_pem)?;

        // Operator (subdirectory).
        std::fs::create_dir_all(tmp.join("operator"))?;
        write_cert(&tmp.join("operator"), "client", &operator_cert_pem)?;
        write_key(&tmp.join("operator"), "client", &operator_key_pem)?;

        // Agents (flat files).
        let mut agent_bundles: Vec<AgentBundle> = Vec::new();
        for (name, cert, key) in &agent_certs {
            write_cert(tmp, name, cert)?;
            write_key(tmp, name, key)?;
            agent_bundles.push(AgentBundle {
                name: name.clone(),
                cert: out_dir.join(format!("{name}.crt.pem")),
                key: out_dir.join(format!("{name}.key.pem")),
            });
        }

        promote(tmp_dir, out_dir)?;

        Ok(PkiBundle {
            out_dir: out_dir.to_path_buf(),
            server_ca_cert: out_dir.join(format!("{server_ca_label}.crt.pem")),
            client_ca_cert: out_dir.join("ca.crt.pem"),
            server_cert: out_dir.join("server.crt.pem"),
            server_key: out_dir.join("server.key.pem"),
            console_cert: out_dir.join("console/client.crt.pem"),
            console_key: out_dir.join("console/client.key.pem"),
            operator_cert: out_dir.join("operator/client.crt.pem"),
            operator_key: out_dir.join("operator/client.key.pem"),
            agents: agent_bundles,
        })
    }

    /// Mint a single new agent cert.
    ///
    /// CF mode (`cf_token` + `cf_zone_id` both `Some`): calls CF Client Certs API.
    /// Self-signed mode (both `None`): signs with internal CA from `ca.key.pem`.
    pub fn add_agent(
        certs_dir: &Path,
        name: &str,
        cf_token: Option<&str>,
        cf_zone_id: Option<&str>,
    ) -> InfraResult<AgentBundle> {
        let tmpl = generate_client_cert_template(name)?;

        let cert_pem = if let (Some(tok), Some(zid)) = (cf_token, cf_zone_id) {
            let csr = tmpl
                .serialize_request_pem()
                .map_err(|e| InfraError::CertificateError(format!("{name} CSR: {e}")))?;
            call_cf_client_cert_api(tok, zid, &csr, 3650)?
        } else {
            let ca = load_internal_ca(certs_dir)?;
            tmpl.serialize_pem_with_signer(&ca)
                .map_err(|e| InfraError::CertificateError(format!("{name} sign: {e}")))?
        };

        let key_pem = tmpl.serialize_private_key_pem();
        write_cert(certs_dir, name, &cert_pem)?;
        write_key(certs_dir, name, &key_pem)?;

        Ok(AgentBundle {
            name: name.to_string(),
            cert: certs_dir.join(format!("{name}.crt.pem")),
            key: certs_dir.join(format!("{name}.key.pem")),
        })
    }

    /// Validate that `cert_pem` and `key_pem` correspond to the same key pair
    /// by comparing SubjectPublicKeyInfo bytes. Returns `Ok(())` if they match.
    pub fn verify_cert_key_pair(cert_pem: &[u8], key_pem: &[u8]) -> InfraResult<()> {
        use rustls_pemfile::certs;
        use x509_parser::prelude::FromDer as _;
        use x509_parser::prelude::X509Certificate;

        let key_str = std::str::from_utf8(key_pem).map_err(|e| {
            InfraError::CertificateError(format!("key PEM not UTF-8: {e}"))
        })?;
        let key_spki = KeyPair::from_pem(key_str)
            .map_err(|e| InfraError::CertificateError(format!("parse private key: {e}")))?
            .public_key_der();

        let cert_ders = certs(&mut std::io::Cursor::new(cert_pem))
            .map_err(|e| InfraError::CertificateError(format!("parse cert PEM: {e}")))?;
        let cert_der = cert_ders.into_iter().next().ok_or_else(|| {
            InfraError::CertificateError("no certificate in PEM".to_string())
        })?;
        let (_, cert) = X509Certificate::from_der(&cert_der)
            .map_err(|e| InfraError::CertificateError(format!("parse cert DER: {e}")))?;

        if key_spki.as_slice() != cert.public_key().raw {
            return Err(InfraError::TlsError(
                "private key does not match certificate (SPKI mismatch)".to_string(),
            ));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Cloudflare API calls
// ---------------------------------------------------------------------------

/// Call `POST https://api.cloudflare.com/client/v4/certificates` (Origin CA).
fn call_cf_origin_ca_api(
    api_token: &str,
    csr_pem: &str,
    hostnames: &[String],
) -> InfraResult<String> {
    use serde_json::{json, Value};
    let client = reqwest::blocking::Client::new();
    let resp: Value = client
        .post("https://api.cloudflare.com/client/v4/certificates")
        .header("Authorization", format!("Bearer {api_token}"))
        .json(&json!({
            "csr": csr_pem,
            "hostnames": hostnames,
            "request_type": "origin-ecc",
            "requested_validity": 5475
        }))
        .send()
        .map_err(InfraError::NetworkError)?
        .json()
        .map_err(|e| InfraError::CertificateError(format!("CF Origin CA response: {e}")))?;

    check_cf_success(&resp, "CF Origin CA")?;
    cf_cert_field(&resp, "CF Origin CA")
}

/// Call `POST https://api.cloudflare.com/client/v4/zones/{zone_id}/client_certificates`.
fn call_cf_client_cert_api(
    api_token: &str,
    zone_id: &str,
    csr_pem: &str,
    validity_days: u32,
) -> InfraResult<String> {
    use serde_json::{json, Value};
    let client = reqwest::blocking::Client::new();
    let url = format!(
        "https://api.cloudflare.com/client/v4/zones/{zone_id}/client_certificates"
    );
    let resp: Value = client
        .post(&url)
        .header("Authorization", format!("Bearer {api_token}"))
        .json(&json!({ "csr": csr_pem, "validity_days": validity_days }))
        .send()
        .map_err(InfraError::NetworkError)?
        .json()
        .map_err(|e| InfraError::CertificateError(format!("CF Client Certs response: {e}")))?;

    check_cf_success(&resp, "CF Client Certs")?;
    cf_cert_field(&resp, "CF Client Certs")
}

fn check_cf_success(resp: &serde_json::Value, label: &str) -> InfraResult<()> {
    if !resp["success"].as_bool().unwrap_or(false) {
        let errs = resp["errors"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|e| {
                        format!(
                            "[{}] {}",
                            e["code"].as_u64().unwrap_or(0),
                            e["message"].as_str().unwrap_or("unknown")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .unwrap_or_else(|| resp["errors"].to_string());
        return Err(InfraError::CertificateError(format!("{label} API error: {errs}")));
    }
    Ok(())
}

fn cf_cert_field(resp: &serde_json::Value, label: &str) -> InfraResult<String> {
    resp["result"]["certificate"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| {
            InfraError::CertificateError(format!("{label}: response missing result.certificate"))
        })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn generate_ca(cn: &str) -> InfraResult<Certificate> {
    let mut params = CertificateParams::new(vec![]);
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, cn);
    dn.push(DnType::OrganizationName, "Nexus C2");
    params.distinguished_name = dn;
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.alg = &PKCS_ECDSA_P256_SHA256;
    params.key_pair = Some(
        KeyPair::generate(&PKCS_ECDSA_P256_SHA256)
            .map_err(|e| InfraError::CertificateError(format!("CA keygen: {e}")))?,
    );
    set_validity(&mut params, 1825);
    Certificate::from_params(params)
        .map_err(|e| InfraError::CertificateError(format!("CA from_params: {e}")))
}

fn generate_server_cert_template(domain: &str, ip: IpAddr) -> InfraResult<Certificate> {
    let mut params = CertificateParams::new(vec![domain.to_string()]);
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, domain);
    params.distinguished_name = dn;
    params.subject_alt_names = vec![
        SanType::DnsName(domain.to_string()),
        SanType::IpAddress(ip),
    ];
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    params.alg = &PKCS_ECDSA_P256_SHA256;
    params.key_pair = Some(
        KeyPair::generate(&PKCS_ECDSA_P256_SHA256)
            .map_err(|e| InfraError::CertificateError(format!("server keygen: {e}")))?,
    );
    set_validity(&mut params, 365);
    Certificate::from_params(params)
        .map_err(|e| InfraError::CertificateError(format!("server template: {e}")))
}

fn generate_client_cert_template(cn: &str) -> InfraResult<Certificate> {
    let mut params = CertificateParams::new(vec![]);
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, cn);
    params.distinguished_name = dn;
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
    params.alg = &PKCS_ECDSA_P256_SHA256;
    params.key_pair = Some(
        KeyPair::generate(&PKCS_ECDSA_P256_SHA256)
            .map_err(|e| InfraError::CertificateError(format!("{cn} keygen: {e}")))?,
    );
    set_validity(&mut params, 365);
    Certificate::from_params(params)
        .map_err(|e| InfraError::CertificateError(format!("{cn} template: {e}")))
}

fn set_validity(params: &mut CertificateParams, days: i64) {
    let now = chrono::Utc::now().timestamp();
    let then = now + days * 86400;
    if let Ok(nb) = ::time::OffsetDateTime::from_unix_timestamp(now) {
        params.not_before = nb;
    }
    if let Ok(na) = ::time::OffsetDateTime::from_unix_timestamp(then) {
        params.not_after = na;
    }
}

fn write_cert(dir: &Path, name: &str, pem: &str) -> InfraResult<()> {
    std::fs::write(dir.join(format!("{name}.crt.pem")), pem)?;
    Ok(())
}

fn write_key(dir: &Path, name: &str, pem: &str) -> InfraResult<()> {
    let path = dir.join(format!("{name}.key.pem"));
    std::fs::write(&path, pem)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn tempfile_dir(target_dir: &Path) -> InfraResult<PathBuf> {
    let parent = target_dir.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    let tmp = parent.join(".nexus-pki-tmp");
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp)?;
    }
    std::fs::create_dir_all(&tmp)?;
    Ok(tmp)
}

fn promote(src: PathBuf, dst: &Path) -> InfraResult<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(&src)? {
        let entry = entry?;
        let dest = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            // Recurse for subdirectories (console/, operator/).
            promote(entry.path(), &dest)?;
        } else {
            std::fs::rename(entry.path(), dest)?;
        }
    }
    let _ = std::fs::remove_dir(&src);
    Ok(())
}

/// Load the internal CA certificate + key from `<dir>/ca.key.pem`.
/// Only valid in self-signed mode (where `ca.key.pem` was written by `init`).
fn load_internal_ca(dir: &Path) -> InfraResult<Certificate> {
    let key_path = dir.join("ca.key.pem");
    let key_pem = std::fs::read_to_string(&key_path).map_err(|e| {
        InfraError::CertificateError(format!(
            "read {}: {e} — is this a CF-mode bundle? Use --cf-token + --cf-zone-id for add_agent",
            key_path.display()
        ))
    })?;
    let key_pair = KeyPair::from_pem(&key_pem)
        .map_err(|e| InfraError::CertificateError(format!("parse CA key: {e}")))?;

    let mut params = CertificateParams::new(vec![]);
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "Nexus C2 CA");
    dn.push(DnType::OrganizationName, "Nexus C2");
    params.distinguished_name = dn;
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.alg = &PKCS_ECDSA_P256_SHA256;
    params.key_pair = Some(key_pair);

    Certificate::from_params(params)
        .map_err(|e| InfraError::CertificateError(format!("CA from_params: {e}")))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn init_self_signed(out: &Path, agents: u8) -> PkiBundle {
        PkiManager::init("test.example.com", "127.0.0.1".parse().unwrap(), agents, out,
                         None, None, None)
            .expect("init self-signed")
    }

    #[test]
    fn self_signed_creates_expected_files() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("certs");
        let b = init_self_signed(&out, 2);

        for p in [
            &b.server_ca_cert, &b.client_ca_cert,
            &b.server_cert, &b.server_key,
            &b.console_cert, &b.console_key,
            &b.operator_cert, &b.operator_key,
        ] {
            assert!(p.exists(), "missing: {}", p.display());
        }
        assert_eq!(b.agents.len(), 2);
        for a in &b.agents {
            assert!(a.cert.exists());
            assert!(a.key.exists());
        }
        // Internal CA key must exist in self-signed mode.
        assert!(out.join("ca.key.pem").exists());
    }

    #[test]
    fn cf_mode_without_client_ca_errors() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("certs");
        let err = PkiManager::init(
            "test.example.com", "127.0.0.1".parse().unwrap(), 0, &out,
            Some("tok"), Some("zone"), None,
        ).unwrap_err();
        assert!(err.to_string().contains("--cf-client-ca"), "{err}");
    }

    #[test]
    fn rejects_existing_bundle() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("certs");
        init_self_signed(&out, 0);
        let err = PkiManager::init(
            "test.example.com", "127.0.0.1".parse().unwrap(), 0, &out,
            None, None, None,
        ).unwrap_err();
        assert!(err.to_string().contains("already initialized"), "{err}");
    }

    #[test]
    fn add_agent_self_signed() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("certs");
        init_self_signed(&out, 0);
        let ab = PkiManager::add_agent(&out, "agent-win01", None, None).expect("add_agent");
        assert!(ab.cert.exists());
        assert!(ab.key.exists());
    }

    #[test]
    fn verify_cert_key_pair_detects_mismatch() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("certs");
        let b = init_self_signed(&out, 1);
        let server_cert = std::fs::read(&b.server_cert).unwrap();
        let agent_key = std::fs::read(&b.agents[0].key).unwrap();
        assert!(PkiManager::verify_cert_key_pair(&server_cert, &agent_key).is_err());
    }

    #[test]
    fn verify_cert_key_pair_accepts_correct_pair() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("certs");
        let b = init_self_signed(&out, 0);
        let cert = std::fs::read(&b.server_cert).unwrap();
        let key = std::fs::read(&b.server_key).unwrap();
        PkiManager::verify_cert_key_pair(&cert, &key).expect("matching pair");
    }

    #[test]
    fn cf_origin_ca_root_parses() {
        use rustls_pemfile::certs;
        let ders = certs(&mut std::io::Cursor::new(CF_ORIGIN_CA_ECC_ROOT.as_bytes())).unwrap();
        assert_eq!(ders.len(), 1);
    }

    #[test]
    fn self_signed_no_ca_key_in_server_ca_path() {
        // In self-signed mode server-ca.crt.pem and ca.crt.pem both exist
        // but server-ca.crt.pem contains the internal CA (not CF root).
        let dir = tempdir().unwrap();
        let out = dir.path().join("certs");
        let b = init_self_signed(&out, 0);
        assert!(b.server_ca_cert.exists());
        // server_ca_cert == ca.crt.pem in self-signed mode (both same file).
        assert_eq!(b.server_ca_cert, b.client_ca_cert);
    }
}
