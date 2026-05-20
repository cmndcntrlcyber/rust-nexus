//! v1.4 Ed25519-signed operator tokens (Phase 1.4.7 / D-V1.4-D, E).
//!
//! Wire format (97 bytes total):
//!
//! ```text
//! +--------+-------------------+-------------+-------------+------------------+
//! | ver(1) | operator_id(16)   | issued (8)  | expires (8) | ed25519_sig(64)  |
//! +--------+-------------------+-------------+-------------+------------------+
//! ```
//!
//! - `ver`: u8 version, currently 1.
//! - `operator_id`: 16-byte opaque identifier (operator's UUID, or a
//!   BLAKE3(cert CN) prefix; v1.4 servers pick the format).
//! - `issued`, `expires`: u64 Unix seconds (big-endian).
//! - `sig`: Ed25519 signature over the preceding 33 bytes,
//!   produced by the server's `NodeIdentity`.
//!
//! Verifiers know the server's Ed25519 public key (the same one that
//! signs `AgentCard`s — see [`crate::cards`]).
//!
//! Tokens are presented to the server on each RPC via gRPC metadata
//! (`x-nexus-operator-token` header, hex-encoded 97 bytes).

use ed25519_dalek::{Signature, Verifier, VerifyingKey, SIGNATURE_LENGTH};
use nexus_common::NodeIdentity;

/// Wire-format version recognised by v1.4.
pub const TOKEN_VERSION: u8 = 1;

/// Total token size in bytes.
pub const TOKEN_LEN: usize = 1 + 16 + 8 + 8 + SIGNATURE_LENGTH;

/// Default token lifetime (24 hours).
pub const DEFAULT_LIFETIME_SECONDS: u64 = 24 * 3600;

/// gRPC metadata header name for token transport.
pub const TOKEN_METADATA_KEY: &str = "x-nexus-operator-token";

/// Errors emitted by encode / decode / verify.
#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    /// Token length wasn't [`TOKEN_LEN`] bytes.
    #[error("token must be {TOKEN_LEN} bytes, got {0}")]
    BadLength(usize),
    /// Token version unrecognised.
    #[error("unknown token version: {0}")]
    BadVersion(u8),
    /// Token's `issued`/`expires` interval was rejected by the verifier.
    #[error("token expired or not-yet-valid: issued={issued} expires={expires} now={now}")]
    Expired {
        /// Unix seconds the token was issued at.
        issued: u64,
        /// Unix seconds the token expires at.
        expires: u64,
        /// Unix seconds at verify time.
        now: u64,
    },
    /// Token signature failed verification.
    #[error("token signature invalid")]
    BadSignature,
    /// Server public key wasn't 32 bytes / didn't decode.
    #[error("server public key invalid: {0}")]
    BadServerKey(String),
}

/// Decoded operator token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorToken {
    /// Wire-format version.
    pub version: u8,
    /// 16-byte operator identifier.
    pub operator_id: [u8; 16],
    /// Unix seconds at issue time.
    pub issued_unix: u64,
    /// Unix seconds at expiry.
    pub expires_unix: u64,
}

impl OperatorToken {
    /// Sign + encode the token using the server's `NodeIdentity`.
    /// Returns the 97-byte wire format.
    pub fn issue(
        identity: &NodeIdentity,
        operator_id: [u8; 16],
        now_unix: u64,
        lifetime_seconds: u64,
    ) -> ([u8; TOKEN_LEN], Self) {
        let token = Self {
            version: TOKEN_VERSION,
            operator_id,
            issued_unix: now_unix,
            expires_unix: now_unix.saturating_add(lifetime_seconds),
        };
        let mut buf = [0u8; TOKEN_LEN];
        let signed = token.fill_signed_prefix(&mut buf);
        let sig = identity.sign(signed);
        buf[33..33 + SIGNATURE_LENGTH].copy_from_slice(&sig);
        (buf, token)
    }

    /// Decode + verify a token against the server's Ed25519 public key.
    pub fn decode_verified(
        bytes: &[u8],
        server_pubkey: &[u8; 32],
        now_unix: u64,
    ) -> Result<Self, TokenError> {
        if bytes.len() != TOKEN_LEN {
            return Err(TokenError::BadLength(bytes.len()));
        }
        let version = bytes[0];
        if version != TOKEN_VERSION {
            return Err(TokenError::BadVersion(version));
        }
        let mut operator_id = [0u8; 16];
        operator_id.copy_from_slice(&bytes[1..17]);
        let mut issued_be = [0u8; 8];
        issued_be.copy_from_slice(&bytes[17..25]);
        let issued_unix = u64::from_be_bytes(issued_be);
        let mut expires_be = [0u8; 8];
        expires_be.copy_from_slice(&bytes[25..33]);
        let expires_unix = u64::from_be_bytes(expires_be);
        let mut sig_bytes = [0u8; SIGNATURE_LENGTH];
        sig_bytes.copy_from_slice(&bytes[33..33 + SIGNATURE_LENGTH]);

        // Lifetime check.
        if now_unix < issued_unix || now_unix >= expires_unix {
            return Err(TokenError::Expired {
                issued: issued_unix,
                expires: expires_unix,
                now: now_unix,
            });
        }

        // Signature verification.
        let signed_prefix = &bytes[..33];
        let vk = VerifyingKey::from_bytes(server_pubkey)
            .map_err(|e| TokenError::BadServerKey(e.to_string()))?;
        let sig = Signature::from_bytes(&sig_bytes);
        vk.verify(signed_prefix, &sig)
            .map_err(|_| TokenError::BadSignature)?;

        Ok(Self {
            version,
            operator_id,
            issued_unix,
            expires_unix,
        })
    }

    /// Fill the signed-bytes prefix (`version || operator_id ||
    /// issued || expires`) into `out` and return the slice.
    fn fill_signed_prefix<'a>(&self, out: &'a mut [u8; TOKEN_LEN]) -> &'a [u8] {
        out[0] = self.version;
        out[1..17].copy_from_slice(&self.operator_id);
        out[17..25].copy_from_slice(&self.issued_unix.to_be_bytes());
        out[25..33].copy_from_slice(&self.expires_unix.to_be_bytes());
        &out[..33]
    }

    /// Encode operator_id as a hex string for log lines.
    pub fn operator_id_hex(&self) -> String {
        let mut s = String::with_capacity(32);
        for b in self.operator_id {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    #[test]
    fn round_trip() {
        let id = NodeIdentity::from_seed(&[7u8; 32]);
        let pubkey = id.ed25519_public();
        let now = now();
        let (bytes, _token) = OperatorToken::issue(&id, [0xaa; 16], now, 3600);
        let decoded = OperatorToken::decode_verified(&bytes, &pubkey, now + 1).expect("verify");
        assert_eq!(decoded.operator_id, [0xaa; 16]);
        assert_eq!(decoded.issued_unix, now);
        assert_eq!(decoded.expires_unix, now + 3600);
    }

    #[test]
    fn expired_rejected() {
        let id = NodeIdentity::from_seed(&[7u8; 32]);
        let pubkey = id.ed25519_public();
        let now = now();
        let (bytes, _) = OperatorToken::issue(&id, [0xaa; 16], now, 60);
        let err =
            OperatorToken::decode_verified(&bytes, &pubkey, now + 120).expect_err("must reject");
        matches!(err, TokenError::Expired { .. });
    }

    #[test]
    fn not_yet_valid_rejected() {
        let id = NodeIdentity::from_seed(&[7u8; 32]);
        let pubkey = id.ed25519_public();
        let now = now();
        let (bytes, _) = OperatorToken::issue(&id, [0xaa; 16], now + 1000, 60);
        let err = OperatorToken::decode_verified(&bytes, &pubkey, now)
            .expect_err("must reject (issued in the future)");
        matches!(err, TokenError::Expired { .. });
    }

    #[test]
    fn tampered_operator_id_rejected() {
        let id = NodeIdentity::from_seed(&[7u8; 32]);
        let pubkey = id.ed25519_public();
        let now = now();
        let (mut bytes, _) = OperatorToken::issue(&id, [0xaa; 16], now, 3600);
        bytes[5] ^= 0xff; // tamper inside operator_id
        let err =
            OperatorToken::decode_verified(&bytes, &pubkey, now + 1).expect_err("must reject");
        matches!(err, TokenError::BadSignature);
    }

    #[test]
    fn signed_by_other_identity_rejected() {
        let signer = NodeIdentity::from_seed(&[7u8; 32]);
        let other = NodeIdentity::from_seed(&[8u8; 32]);
        let other_pubkey = other.ed25519_public();
        let now = now();
        let (bytes, _) = OperatorToken::issue(&signer, [0xaa; 16], now, 3600);
        let err = OperatorToken::decode_verified(&bytes, &other_pubkey, now + 1)
            .expect_err("must reject");
        matches!(err, TokenError::BadSignature);
    }

    #[test]
    fn bad_length_rejected() {
        let pubkey = NodeIdentity::from_seed(&[7u8; 32]).ed25519_public();
        let err = OperatorToken::decode_verified(&[0u8; 96], &pubkey, 0).expect_err("too short");
        matches!(err, TokenError::BadLength(96));
    }

    #[test]
    fn bad_version_rejected() {
        let id = NodeIdentity::from_seed(&[7u8; 32]);
        let pubkey = id.ed25519_public();
        let now = now();
        let (mut bytes, _) = OperatorToken::issue(&id, [0xaa; 16], now, 3600);
        bytes[0] = 2; // unsupported version
        let err =
            OperatorToken::decode_verified(&bytes, &pubkey, now + 1).expect_err("must reject");
        matches!(err, TokenError::BadVersion(2));
    }

    #[test]
    fn operator_id_hex_format() {
        let token = OperatorToken {
            version: 1,
            operator_id: [0xab; 16],
            issued_unix: 0,
            expires_unix: 1,
        };
        let hex = token.operator_id_hex();
        assert_eq!(hex.len(), 32);
        assert!(hex.chars().all(|c| c == 'a' || c == 'b'));
    }
}
