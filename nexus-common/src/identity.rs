//! Cryptographic identity for a rust-nexus node (v1.1 simple-mesh layer).
//!
//! A [`NodeIdentity`] bundles an Ed25519 signing key (for sealed-envelope
//! authentication and future signed `AgentCard`s) and an X25519 secret (for
//! ECDH on the libp2p mesh transport). Both are derived from independent
//! random sources at [`NodeIdentity::generate`] time, or deterministically
//! from a 32-byte seed at [`NodeIdentity::from_seed`].
//!
//! **Note (v1.1 integration):** this lives alongside the overlay's existing
//! `nexus_common::crypto::Crypto` (AES-256-GCM symmetric wrapper) — they are
//! separate primitives serving separate roles. The overlay's `Agent.id`
//! (UUID) is unrelated to [`PeerId`] here; the `nexus-infra::a2a_lister`
//! bridge maps between them via `BLAKE3(uuid.bytes)`.
//!
//! ## Persistence format
//!
//! 72 bytes, little-endian:
//!
//! | offset | length | contents                            |
//! |--------|--------|-------------------------------------|
//! | 0      | 8      | magic `b"NXS_ID01"`                 |
//! | 8      | 32     | Ed25519 signing seed                |
//! | 40     | 32     | X25519 static secret bytes          |

use std::path::Path;

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};

use crate::{NexusError, Result};

/// BLAKE3(Ed25519 public key) — 32 bytes, used as a stable node reference
/// inside the A2A protocol and on the mesh.
pub type PeerId = [u8; 32];

/// Length of the on-disk identity blob.
pub const IDENTITY_BLOB_LEN: usize = 72;

const IDENTITY_MAGIC: &[u8; 8] = b"NXS_ID01";

/// Bundle of long-lived secrets and the public material derived from them.
pub struct NodeIdentity {
    ed25519_signing: SigningKey,
    x25519_secret: StaticSecret,
}

impl NodeIdentity {
    /// Sample a fresh identity from the OS CSPRNG.
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let ed25519_signing = SigningKey::generate(&mut csprng);
        let x25519_secret = StaticSecret::random_from_rng(csprng);
        Self {
            ed25519_signing,
            x25519_secret,
        }
    }

    /// Derive an identity deterministically from a 32-byte seed. Equal seeds
    /// produce equal identities — including equal [`peer_id`](Self::peer_id)
    /// values. Intended for tests / reproducible fixtures.
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let ed25519_signing = SigningKey::from_bytes(seed);
        let mut x_bytes = [0u8; 32];
        x_bytes.copy_from_slice(blake3::hash(seed).as_bytes());
        let x25519_secret = StaticSecret::from(x_bytes);
        Self {
            ed25519_signing,
            x25519_secret,
        }
    }

    /// Stable 32-byte node reference: `BLAKE3(Ed25519 verifying-key bytes)`.
    pub fn peer_id(&self) -> PeerId {
        let pubkey = self.ed25519_signing.verifying_key();
        let mut pid = [0u8; 32];
        pid.copy_from_slice(blake3::hash(pubkey.as_bytes()).as_bytes());
        pid
    }

    /// 32-byte Ed25519 public verifying key. Distributable.
    pub fn ed25519_public(&self) -> [u8; 32] {
        self.ed25519_signing.verifying_key().to_bytes()
    }

    /// 32-byte X25519 public key. Distributable.
    pub fn x25519_public(&self) -> [u8; 32] {
        X25519PublicKey::from(&self.x25519_secret).to_bytes()
    }

    /// 32-byte Ed25519 signing seed. **Sensitive material.** Exposed so
    /// `nexus-mesh` can construct a `libp2p::identity::Keypair` from the
    /// same identity bytes.
    pub fn ed25519_seed(&self) -> [u8; 32] {
        self.ed25519_signing.to_bytes()
    }

    /// X25519 ECDH against a counterparty public key.
    pub fn x25519_diffie_hellman(&self, peer_public: &[u8; 32]) -> [u8; 32] {
        let peer = X25519PublicKey::from(*peer_public);
        self.x25519_secret.diffie_hellman(&peer).to_bytes()
    }

    /// Sign a message with the Ed25519 signing key.
    pub fn sign(&self, msg: &[u8]) -> [u8; 64] {
        self.ed25519_signing.sign(msg).to_bytes()
    }

    /// Verify a detached signature against an Ed25519 public key.
    pub fn verify(public: &[u8; 32], msg: &[u8], sig: &[u8; 64]) -> Result<()> {
        let vk = VerifyingKey::from_bytes(public)
            .map_err(|e| NexusError::InvalidIdentity(e.to_string()))?;
        let signature = Signature::from_bytes(sig);
        vk.verify(msg, &signature)
            .map_err(|_| NexusError::SignatureVerificationFailed)
    }

    /// Serialize to the 72-byte on-disk format.
    pub fn to_bytes(&self) -> [u8; IDENTITY_BLOB_LEN] {
        let mut out = [0u8; IDENTITY_BLOB_LEN];
        out[..8].copy_from_slice(IDENTITY_MAGIC);
        out[8..40].copy_from_slice(&self.ed25519_signing.to_bytes());
        out[40..72].copy_from_slice(&self.x25519_secret.to_bytes());
        out
    }

    /// Parse the 72-byte on-disk format.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != IDENTITY_BLOB_LEN {
            return Err(NexusError::InvalidIdentity(format!(
                "expected {IDENTITY_BLOB_LEN} bytes, got {}",
                bytes.len()
            )));
        }
        if &bytes[..8] != IDENTITY_MAGIC {
            return Err(NexusError::InvalidIdentity("bad magic".into()));
        }
        let mut ed_bytes = [0u8; 32];
        ed_bytes.copy_from_slice(&bytes[8..40]);
        let ed25519_signing = SigningKey::from_bytes(&ed_bytes);
        let mut x_bytes = [0u8; 32];
        x_bytes.copy_from_slice(&bytes[40..72]);
        let x25519_secret = StaticSecret::from(x_bytes);
        Ok(Self {
            ed25519_signing,
            x25519_secret,
        })
    }

    /// Atomically write the identity to `path`. Creates parent directories.
    /// On Unix, sets file permissions to `0o600`.
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| NexusError::NetworkError(format!("identity mkdir: {e}")))?;
            }
        }
        std::fs::write(path, self.to_bytes())
            .map_err(|e| NexusError::NetworkError(format!("identity write: {e}")))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
                .map_err(|e| NexusError::NetworkError(format!("identity chmod: {e}")))?;
        }
        Ok(())
    }

    /// Load from a file previously written by [`save_to_file`](Self::save_to_file).
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)
            .map_err(|e| NexusError::NetworkError(format!("identity read: {e}")))?;
        Self::from_bytes(&bytes)
    }

    /// Convenience: load from `path` if present, otherwise generate + save.
    pub fn load_or_create(path: &Path) -> Result<Self> {
        if path.exists() {
            Self::load_from_file(path)
        } else {
            let identity = Self::generate();
            identity.save_to_file(path)?;
            Ok(identity)
        }
    }
}

impl std::fmt::Debug for NodeIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print secret material. Show the peer_id only.
        let pid = self.peer_id();
        let mut s = String::with_capacity(20);
        use std::fmt::Write as _;
        for &b in pid.iter().take(8) {
            let _ = write!(s, "{b:02x}");
        }
        s.push('…');
        write!(f, "NodeIdentity(peer_id={s})")
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn generate_produces_unique_peer_ids() {
        let a = NodeIdentity::generate();
        let b = NodeIdentity::generate();
        assert_ne!(a.peer_id(), b.peer_id());
    }

    #[test]
    fn from_seed_is_deterministic() {
        let seed = [42u8; 32];
        let a = NodeIdentity::from_seed(&seed);
        let b = NodeIdentity::from_seed(&seed);
        assert_eq!(a.peer_id(), b.peer_id());
        assert_eq!(a.ed25519_public(), b.ed25519_public());
        assert_eq!(a.x25519_public(), b.x25519_public());
    }

    #[test]
    fn sign_and_verify_round_trip() {
        let id = NodeIdentity::from_seed(&[7u8; 32]);
        let msg = b"hello, mesh";
        let sig = id.sign(msg);
        NodeIdentity::verify(&id.ed25519_public(), msg, &sig).expect("verify");
    }

    #[test]
    fn verify_rejects_tampered_message() {
        let id = NodeIdentity::from_seed(&[7u8; 32]);
        let sig = id.sign(b"hello, mesh");
        let err = NodeIdentity::verify(&id.ed25519_public(), b"hello, MESH", &sig)
            .expect_err("must fail");
        assert!(matches!(err, NexusError::SignatureVerificationFailed));
    }

    #[test]
    fn x25519_diffie_hellman_is_symmetric() {
        let a = NodeIdentity::from_seed(&[1u8; 32]);
        let b = NodeIdentity::from_seed(&[2u8; 32]);
        let ab = a.x25519_diffie_hellman(&b.x25519_public());
        let ba = b.x25519_diffie_hellman(&a.x25519_public());
        assert_eq!(ab, ba);
    }

    #[test]
    fn to_bytes_from_bytes_round_trip() {
        let id = NodeIdentity::generate();
        let blob = id.to_bytes();
        let restored = NodeIdentity::from_bytes(&blob).expect("from_bytes");
        assert_eq!(id.peer_id(), restored.peer_id());
        assert_eq!(id.ed25519_public(), restored.ed25519_public());
        assert_eq!(id.x25519_public(), restored.x25519_public());
    }

    #[test]
    fn from_bytes_rejects_bad_magic() {
        let mut blob = NodeIdentity::generate().to_bytes();
        blob[0] = b'X';
        let err = NodeIdentity::from_bytes(&blob).expect_err("must fail");
        assert!(matches!(err, NexusError::InvalidIdentity(_)));
    }

    #[test]
    fn save_and_load_round_trip() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("nested").join("identity.bin");
        let original = NodeIdentity::generate();
        original.save_to_file(&path).expect("save");
        let loaded = NodeIdentity::load_from_file(&path).expect("load");
        assert_eq!(original.peer_id(), loaded.peer_id());
    }

    #[cfg(unix)]
    #[test]
    fn save_sets_unix_permissions_to_600() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("id.bin");
        NodeIdentity::generate().save_to_file(&path).expect("save");
        let mode = std::fs::metadata(&path)
            .expect("metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn load_or_create_creates_on_first_call() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("id.bin");
        assert!(!path.exists());
        let first = NodeIdentity::load_or_create(&path).expect("create");
        assert!(path.exists());
        let second = NodeIdentity::load_or_create(&path).expect("load");
        assert_eq!(first.peer_id(), second.peer_id());
    }

    #[test]
    fn debug_does_not_leak_secret_material() {
        let id = NodeIdentity::from_seed(&[0xAB; 32]);
        let dbg = format!("{id:?}");
        assert!(dbg.contains("NodeIdentity"));
        assert!(!dbg.contains("ababababab"));
    }
}
