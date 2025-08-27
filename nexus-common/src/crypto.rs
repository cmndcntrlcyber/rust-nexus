use crate::{NexusError, Result};
use aes_gcm::{Aes256Gcm, Key, Nonce, aead::{Aead, NewAead}};
use base64::{Engine as _, engine::general_purpose};
use rand::Rng;

pub struct Crypto {
    key: [u8; 32],
}

impl Crypto {
    pub fn new(key: [u8; 32]) -> Self {
        Self { key }
    }

    pub fn generate_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        rand::thread_rng().fill(&mut key);
        key
    }

    pub fn encrypt(&self, data: &str) -> Result<String> {
        let key = Key::from_slice(&self.key);
        let cipher = Aes256Gcm::new(key);
        
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = cipher
            .encrypt(nonce, data.as_bytes())
            .map_err(|e| NexusError::EncryptionError(e.to_string()))?;
        
        let mut result = Vec::with_capacity(nonce_bytes.len() + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        
        Ok(general_purpose::STANDARD.encode(result))
    }

    pub fn decrypt(&self, data: &str) -> Result<String> {
        let decoded = general_purpose::STANDARD
            .decode(data)
            .map_err(|e| NexusError::DecryptionError(e.to_string()))?;

        if decoded.len() < 12 {
            return Err(NexusError::DecryptionError("Invalid ciphertext length".to_string()));
        }

        let (nonce_bytes, ciphertext) = decoded.split_at(12);
        let key = Key::from_slice(&self.key);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| NexusError::DecryptionError(e.to_string()))?;
        
        String::from_utf8(plaintext)
            .map_err(|e| NexusError::DecryptionError(e.to_string()))
    }
}

impl Clone for Crypto {
    fn clone(&self) -> Self {
        Self { key: self.key }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let crypto = Crypto::new(Crypto::generate_key());
        let original = "Hello, World!";
        
        let encrypted = crypto.encrypt(original).unwrap();
        let decrypted = crypto.decrypt(&encrypted).unwrap();
        
        assert_eq!(original, decrypted);
    }

    #[test]
    fn test_invalid_decrypt() {
        let crypto = Crypto::new(Crypto::generate_key());
        let result = crypto.decrypt("invalid_base64");
        assert!(result.is_err());
    }
}
