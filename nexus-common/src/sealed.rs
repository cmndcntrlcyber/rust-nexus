//! `SealedEnvelope` — end-to-end encrypted, sender-authenticated payload
//! that rides over the libp2p mesh's gossipsub layer.
//!
//! Wire format (bincode-serialized):
//!
//! ```text
//!     +---------------------- 32 bytes ----------------------+
//!     |  sender_ed25519  (sender's stable Ed25519 public)    |
//!     +---------------------- 32 bytes ----------------------+
//!     |  ephemeral_x25519 (sender's per-message X25519 pub)  |
//!     +---------------------- 12 bytes ----------------------+
//!     |  nonce  (AES-GCM IV)                                 |
//!     +---------------------- variable ----------------------+
//!     |  ciphertext  (AES-256-GCM ENC || 16-byte tag)        |
//!     +---------------------- 64 bytes ----------------------+
//!     |  signature  (Ed25519 over the first three sections + |
//!     |              ciphertext)                             |
//!     +------------------------------------------------------+
//! ```
//!
//! **v1.1 integration note:** uses `aes_gcm::Aes256Gcm` directly (the same
//! crate the overlay's `crypto::Crypto` uses) — we don't go through that
//! wrapper because we need explicit nonce + AAD control which it doesn't
//! expose. The two crypto entry points coexist.

use std::collections::{HashMap, VecDeque};

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use hkdf::Hkdf;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use sha2::Sha256;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};

use crate::identity::NodeIdentity;
use crate::{NexusError, Result};

/// HKDF info tag — bumping this changes the derived-key namespace.
pub const HKDF_INFO: &[u8] = b"nexus-mesh/sealed-envelope/v1";

/// Length of the AES-256 key in bytes.
pub const KEY_LEN: usize = 32;
/// Length of the GCM nonce in bytes.
pub const NONCE_LEN: usize = 12;
/// Length of the GCM authentication tag in bytes (appended to ciphertext).
pub const TAG_LEN: usize = 16;

/// Default per-sender replay-window capacity.
pub const DEFAULT_REPLAY_WINDOW: usize = 1024;

/// One sealed envelope. `bincode`-serializable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedEnvelope {
    /// Sender's stable Ed25519 public key (32 bytes).
    pub sender_ed25519: [u8; 32],
    /// Sender's per-message ephemeral X25519 public key (32 bytes).
    pub ephemeral_x25519: [u8; 32],
    /// AES-256-GCM nonce (12 bytes).
    pub nonce: [u8; NONCE_LEN],
    /// AES-256-GCM ciphertext + 16-byte tag.
    pub ciphertext: Vec<u8>,
    /// Ed25519 signature over `sender_ed25519 || ephemeral_x25519 || nonce ||
    /// ciphertext`.
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
}

impl SealedEnvelope {
    /// Encrypt + sign `plaintext` for `recipient_x25519_pub`.
    pub fn seal(
        sender: &NodeIdentity,
        recipient_x25519_pub: &[u8; 32],
        plaintext: &[u8],
    ) -> Result<Self> {
        let mut rng = OsRng;
        let ephemeral_secret = StaticSecret::random_from_rng(rng);
        let ephemeral_pub = X25519PublicKey::from(&ephemeral_secret);

        let recipient_pub = X25519PublicKey::from(*recipient_x25519_pub);
        let shared = ephemeral_secret.diffie_hellman(&recipient_pub);

        let key = derive_key(shared.as_bytes())?;

        let mut nonce = [0u8; NONCE_LEN];
        rng.fill_bytes(&mut nonce);

        let ciphertext = aes256gcm_encrypt(&key, &nonce, plaintext, b"")?;

        let sender_ed25519 = sender.ed25519_public();
        let ephemeral_x25519 = ephemeral_pub.to_bytes();
        let signed = canonical_bytes(&sender_ed25519, &ephemeral_x25519, &nonce, &ciphertext);
        let signature = sender.sign(&signed);

        Ok(Self {
            sender_ed25519,
            ephemeral_x25519,
            nonce,
            ciphertext,
            signature,
        })
    }

    /// Verify the signature and decrypt with `recipient`'s X25519 secret.
    pub fn open(&self, recipient: &NodeIdentity) -> Result<Vec<u8>> {
        let signed = canonical_bytes(
            &self.sender_ed25519,
            &self.ephemeral_x25519,
            &self.nonce,
            &self.ciphertext,
        );
        NodeIdentity::verify(&self.sender_ed25519, &signed, &self.signature)?;

        let shared = recipient.x25519_diffie_hellman(&self.ephemeral_x25519);
        let key = derive_key(&shared)?;

        aes256gcm_decrypt(&key, &self.nonce, &self.ciphertext, b"")
    }

    /// Serialize to bincode bytes for gossipsub publication.
    pub fn to_bincode(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| NexusError::BincodeError(e.to_string()))
    }

    /// Deserialize from bincode bytes.
    pub fn from_bincode(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).map_err(|e| NexusError::BincodeError(e.to_string()))
    }
}

fn derive_key(ikm: &[u8; 32]) -> Result<[u8; KEY_LEN]> {
    let hkdf = Hkdf::<Sha256>::new(None, ikm);
    let mut key = [0u8; KEY_LEN];
    hkdf.expand(HKDF_INFO, &mut key)
        .map_err(|e| NexusError::CryptoFailure(format!("hkdf expand: {e}")))?;
    Ok(key)
}

fn canonical_bytes(
    sender: &[u8; 32],
    ephemeral: &[u8; 32],
    nonce: &[u8; NONCE_LEN],
    ct: &[u8],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(32 + 32 + NONCE_LEN + ct.len());
    out.extend_from_slice(sender);
    out.extend_from_slice(ephemeral);
    out.extend_from_slice(nonce);
    out.extend_from_slice(ct);
    out
}

/// AES-256-GCM encrypt with explicit nonce + AAD. Sibling to the overlay's
/// `crypto::Crypto::encrypt` (which uses a different nonce / payload shape).
pub fn aes256gcm_encrypt(
    key: &[u8; KEY_LEN],
    nonce: &[u8; NONCE_LEN],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| NexusError::CryptoFailure(format!("aes256gcm key: {e}")))?;
    let nonce = Nonce::from_slice(nonce);
    cipher
        .encrypt(
            nonce,
            aes_gcm::aead::Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|e| NexusError::CryptoFailure(format!("aes256gcm encrypt: {e}")))
}

/// AES-256-GCM decrypt + authenticate.
pub fn aes256gcm_decrypt(
    key: &[u8; KEY_LEN],
    nonce: &[u8; NONCE_LEN],
    ciphertext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| NexusError::CryptoFailure(format!("aes256gcm key: {e}")))?;
    let nonce = Nonce::from_slice(nonce);
    cipher
        .decrypt(
            nonce,
            aes_gcm::aead::Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|e| NexusError::CryptoFailure(format!("aes256gcm decrypt: {e}")))
}

/// Per-sender sliding-window replay cache.
pub struct ReplayWindow {
    seen: HashMap<[u8; 32], VecDeque<[u8; NONCE_LEN]>>,
    capacity_per_sender: usize,
}

impl ReplayWindow {
    /// New empty window.
    #[must_use]
    pub fn new(capacity_per_sender: usize) -> Self {
        Self {
            seen: HashMap::new(),
            capacity_per_sender,
        }
    }

    /// `true` if `nonce` is fresh for `sender`. Records the nonce. `false`
    /// for replays.
    pub fn check_and_record(&mut self, sender: &[u8; 32], nonce: &[u8; NONCE_LEN]) -> bool {
        let entry = self.seen.entry(*sender).or_default();
        if entry.contains(nonce) {
            return false;
        }
        if entry.len() >= self.capacity_per_sender {
            entry.pop_front();
        }
        entry.push_back(*nonce);
        true
    }

    /// Drop all state for `sender`.
    pub fn forget(&mut self, sender: &[u8; 32]) {
        self.seen.remove(sender);
    }

    /// Current per-sender slot counts.
    pub fn len_for(&self, sender: &[u8; 32]) -> usize {
        self.seen.get(sender).map_or(0, VecDeque::len)
    }
}

impl Default for ReplayWindow {
    fn default() -> Self {
        Self::new(DEFAULT_REPLAY_WINDOW)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::NodeIdentity;

    fn alice() -> NodeIdentity {
        NodeIdentity::from_seed(&[1u8; 32])
    }
    fn bob() -> NodeIdentity {
        NodeIdentity::from_seed(&[2u8; 32])
    }
    fn carol() -> NodeIdentity {
        NodeIdentity::from_seed(&[3u8; 32])
    }

    #[test]
    fn round_trip_alice_to_bob() {
        let a = alice();
        let b = bob();
        let env = SealedEnvelope::seal(&a, &b.x25519_public(), b"hello, bob").expect("seal");
        let pt = env.open(&b).expect("open");
        assert_eq!(pt, b"hello, bob");
    }

    #[test]
    fn third_party_cannot_decrypt() {
        let a = alice();
        let b = bob();
        let c = carol();
        let env = SealedEnvelope::seal(&a, &b.x25519_public(), b"top secret").expect("seal");
        let err = env.open(&c).expect_err("must fail");
        assert!(matches!(err, NexusError::CryptoFailure(_)));
    }

    #[test]
    fn tampered_ciphertext_rejected() {
        let a = alice();
        let b = bob();
        let mut env = SealedEnvelope::seal(&a, &b.x25519_public(), b"hello, bob").expect("seal");
        env.ciphertext[0] ^= 0x01;
        let err = env.open(&b).expect_err("must fail");
        assert!(matches!(err, NexusError::SignatureVerificationFailed));
    }

    #[test]
    fn tampered_signature_rejected() {
        let a = alice();
        let b = bob();
        let mut env = SealedEnvelope::seal(&a, &b.x25519_public(), b"hello").expect("seal");
        env.signature[0] ^= 0xff;
        let err = env.open(&b).expect_err("must fail");
        assert!(matches!(err, NexusError::SignatureVerificationFailed));
    }

    #[test]
    fn forged_sender_rejected() {
        let a = alice();
        let b = bob();
        let c = carol();
        let mut env = SealedEnvelope::seal(&a, &b.x25519_public(), b"hi").expect("seal");
        env.sender_ed25519 = c.ed25519_public();
        let err = env.open(&b).expect_err("must fail");
        assert!(matches!(err, NexusError::SignatureVerificationFailed));
    }

    #[test]
    fn aes256gcm_round_trip_with_aad() {
        let key = [7u8; KEY_LEN];
        let nonce = [3u8; NONCE_LEN];
        let pt = b"the quick brown fox";
        let aad = b"associated-metadata-v1";
        let ct = aes256gcm_encrypt(&key, &nonce, pt, aad).expect("encrypt");
        assert_eq!(ct.len(), pt.len() + TAG_LEN);
        let back = aes256gcm_decrypt(&key, &nonce, &ct, aad).expect("decrypt");
        assert_eq!(back, pt);
    }

    #[test]
    fn aes256gcm_tampered_aad_fails() {
        let key = [9u8; KEY_LEN];
        let nonce = [1u8; NONCE_LEN];
        let ct = aes256gcm_encrypt(&key, &nonce, b"hello", b"aad-A").expect("encrypt");
        let err = aes256gcm_decrypt(&key, &nonce, &ct, b"aad-B").expect_err("must fail");
        assert!(matches!(err, NexusError::CryptoFailure(_)));
    }

    #[test]
    fn replay_window_detects_duplicates() {
        let mut win = ReplayWindow::new(4);
        let sender = [1u8; 32];
        let nonce = [9u8; NONCE_LEN];
        assert!(win.check_and_record(&sender, &nonce));
        assert!(!win.check_and_record(&sender, &nonce));
    }

    #[test]
    fn replay_window_evicts_oldest() {
        let mut win = ReplayWindow::new(2);
        let sender = [1u8; 32];
        let n1 = [1u8; NONCE_LEN];
        let n2 = [2u8; NONCE_LEN];
        let n3 = [3u8; NONCE_LEN];
        assert!(win.check_and_record(&sender, &n1));
        assert!(win.check_and_record(&sender, &n2));
        assert!(win.check_and_record(&sender, &n3));
        assert!(win.check_and_record(&sender, &n1));
    }

    #[test]
    fn bincode_round_trip() {
        let a = alice();
        let b = bob();
        let env = SealedEnvelope::seal(&a, &b.x25519_public(), b"hi").expect("seal");
        let bytes = env.to_bincode().expect("encode");
        let back = SealedEnvelope::from_bincode(&bytes).expect("decode");
        assert_eq!(env, back);
    }
}
