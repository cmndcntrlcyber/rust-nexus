//! v1.2 — Ed25519-signed AgentCards (D-V1.2-cards).
//!
//! ## Wire layout
//!
//! The proto `AgentCard` message gains two optional fields:
//!
//! - `signature: bytes` (64 bytes when present) — Ed25519 signature over
//!   the canonical JSON encoding of the unsigned card.
//! - `signer_peer_id: bytes` (32 bytes when present) — the signer's Ed25519
//!   public key, which doubles as their [`nexus_common::PeerId`].
//!
//! Empty bytes in either field = unsigned. Production servers always sign;
//! verification on the client side rejects empty/tampered signatures unless
//! `accept_unsigned_cards = true` is opted in (used by the loopback
//! example and v1.1-compatibility callers).
//!
//! ## Canonical encoding
//!
//! Signed bytes are a stable JSON encoding of the unsigned card:
//!
//! ```json
//! {
//!   "name": "...",
//!   "description": "...",
//!   "version": "...",
//!   "skills": [{"id": "...", "name": "...", "description": "...", "tags": ["..."]}]
//! }
//! ```
//!
//! Field order is fixed (alphabetical within each object), JSON-encoded
//! without whitespace, UTF-8 bytes. The skills array preserves the order
//! given by the signer; verifiers must NOT reorder.

use ed25519_dalek::{Signature, Verifier, VerifyingKey, SIGNATURE_LENGTH};
use nexus_common::NodeIdentity;
use serde::Serialize;

use crate::pb;

/// Errors emitted by sign / verify.
#[derive(Debug, thiserror::Error)]
pub enum CardError {
    /// Signature length wasn't 64 bytes.
    #[error("signature must be {SIGNATURE_LENGTH} bytes, got {0}")]
    BadSignatureLength(usize),
    /// signer_peer_id wasn't 32 bytes.
    #[error("signer_peer_id must be 32 bytes, got {0}")]
    BadSignerLength(usize),
    /// Ed25519 verification rejected the signature.
    #[error("signature verification failed")]
    BadSignature,
    /// The card has empty signature/signer_peer_id when the verifier
    /// requires a signed card.
    #[error("card is unsigned but verifier requires a signature")]
    Unsigned,
    /// JSON encoding error.
    #[error("canonical-json encode: {0}")]
    Encode(#[from] serde_json::Error),
    /// Public key didn't parse as Ed25519.
    #[error("invalid signer_peer_id (Ed25519 public key bytes): {0}")]
    BadPublicKey(String),
}

/// Sign `card` in place with `identity`. The function clears any existing
/// signature/signer fields before computing the canonical encoding, so
/// re-signing is idempotent.
///
/// Note: the `signer_peer_id` proto field carries the raw Ed25519 public
/// key (32 bytes) — the same bytes a verifier needs to call
/// [`ed25519_dalek::Verifier::verify`]. The agent's `peer_id` is
/// `BLAKE3(pubkey)`, derivable from these bytes by anyone who wants the
/// short-form identifier.
pub fn sign(card: &mut pb::AgentCard, identity: &NodeIdentity) {
    card.signature = Vec::new();
    card.signer_peer_id = Vec::new();
    let bytes = canonical_bytes(card).expect("canonical encoding never fails for AgentCard");
    let sig = identity.sign(&bytes);
    card.signature = sig.to_vec();
    card.signer_peer_id = identity.ed25519_public().to_vec();
}

/// Verify `card`'s signature. Returns `Ok(())` on success, an error on
/// length / signature / unsigned mismatch.
pub fn verify(card: &pb::AgentCard) -> Result<(), CardError> {
    if card.signature.is_empty() && card.signer_peer_id.is_empty() {
        return Err(CardError::Unsigned);
    }
    if card.signature.len() != SIGNATURE_LENGTH {
        return Err(CardError::BadSignatureLength(card.signature.len()));
    }
    if card.signer_peer_id.len() != 32 {
        return Err(CardError::BadSignerLength(card.signer_peer_id.len()));
    }
    let mut pk_bytes = [0u8; 32];
    pk_bytes.copy_from_slice(&card.signer_peer_id);
    let vk =
        VerifyingKey::from_bytes(&pk_bytes).map_err(|e| CardError::BadPublicKey(e.to_string()))?;
    let mut sig_bytes = [0u8; SIGNATURE_LENGTH];
    sig_bytes.copy_from_slice(&card.signature);
    let sig = Signature::from_bytes(&sig_bytes);

    // Recompute canonical bytes with the signature fields zeroed.
    let mut probe = card.clone();
    probe.signature = Vec::new();
    probe.signer_peer_id = Vec::new();
    let bytes = canonical_bytes(&probe)?;
    vk.verify(&bytes, &sig).map_err(|_| CardError::BadSignature)
}

/// Canonical-JSON encoding of `card` with signature fields zeroed.
fn canonical_bytes(card: &pb::AgentCard) -> Result<Vec<u8>, serde_json::Error> {
    let view = CanonicalCard {
        description: &card.description,
        name: &card.name,
        skills: card
            .skills
            .iter()
            .map(|s| CanonicalSkill {
                description: &s.description,
                id: &s.id,
                name: &s.name,
                tags: &s.tags,
            })
            .collect(),
        version: &card.version,
    };
    serde_json::to_vec(&view)
}

#[derive(Serialize)]
struct CanonicalCard<'a> {
    description: &'a str,
    name: &'a str,
    skills: Vec<CanonicalSkill<'a>>,
    version: &'a str,
}

#[derive(Serialize)]
struct CanonicalSkill<'a> {
    description: &'a str,
    id: &'a str,
    name: &'a str,
    tags: &'a [String],
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_card() -> pb::AgentCard {
        pb::AgentCard {
            name: "test".into(),
            description: "test card".into(),
            version: "1.0".into(),
            skills: vec![pb::AgentSkill {
                id: "shell-session".into(),
                name: "shell-session".into(),
                description: "...".into(),
                tags: vec!["v1.2".into()],
            }],
            signature: Vec::new(),
            signer_peer_id: Vec::new(),
        }
    }

    #[test]
    fn sign_verify_round_trip() {
        let id = NodeIdentity::from_seed(&[7u8; 32]);
        let mut card = sample_card();
        sign(&mut card, &id);
        assert!(!card.signature.is_empty());
        assert_eq!(card.signer_peer_id.len(), 32);
        verify(&card).expect("round trip");
    }

    #[test]
    fn tampered_name_rejected() {
        let id = NodeIdentity::from_seed(&[7u8; 32]);
        let mut card = sample_card();
        sign(&mut card, &id);
        card.name = "tampered".into();
        let err = verify(&card).expect_err("must reject");
        matches!(err, CardError::BadSignature);
    }

    #[test]
    fn tampered_skill_rejected() {
        let id = NodeIdentity::from_seed(&[7u8; 32]);
        let mut card = sample_card();
        sign(&mut card, &id);
        card.skills.push(pb::AgentSkill {
            id: "injected".into(),
            name: "injected".into(),
            description: "".into(),
            tags: vec![],
        });
        verify(&card).expect_err("must reject extra skill");
    }

    #[test]
    fn unsigned_card_rejected() {
        let card = sample_card();
        let err = verify(&card).expect_err("must reject unsigned");
        matches!(err, CardError::Unsigned);
    }

    #[test]
    fn re_sign_idempotent() {
        let id = NodeIdentity::from_seed(&[7u8; 32]);
        let mut card = sample_card();
        sign(&mut card, &id);
        let first = card.signature.clone();
        sign(&mut card, &id);
        assert_eq!(first, card.signature, "deterministic re-sign");
    }

    #[test]
    fn signer_mismatch_rejected() {
        let id_a = NodeIdentity::from_seed(&[1u8; 32]);
        let id_b = NodeIdentity::from_seed(&[2u8; 32]);
        let mut card = sample_card();
        sign(&mut card, &id_a);
        // Replace the signer with B's peer id but keep A's signature.
        card.signer_peer_id = id_b.peer_id().to_vec();
        verify(&card).expect_err("must reject signer/signature mismatch");
    }
}
