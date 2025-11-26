//! AGE encryption support for config secrets
//!
//! This module provides encryption and decryption of secrets using the AGE
//! (Actually Good Encryption) library with passphrase-based encryption.
//!
//! # Usage in Config
//!
//! Encrypted secrets can be embedded in the config using the following format:
//! ```toml
//! [secrets]
//! openai_api_key = "age:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IHNjcnlwdC..."
//! ```
//!
//! # CLI Commands
//!
//! Encrypt a value:
//! ```sh
//! metis encrypt-secret "my-api-key-value"
//! # Will prompt for passphrase and output: age:base64_encoded_ciphertext
//! ```
//!
//! # Decryption
//!
//! Secrets are decrypted at runtime using:
//! 1. Passphrase from METIS_SECRET_PASSPHRASE environment variable
//! 2. Or, passphrase from --secret-passphrase CLI flag

use std::io::{Read, Write};

const AGE_PREFIX: &str = "age:";

/// Check if a value is AGE-encrypted
pub fn is_encrypted(value: &str) -> bool {
    value.starts_with(AGE_PREFIX)
}

/// Encrypt a secret value with a passphrase using AGE
///
/// Returns a string in the format "age:base64_encoded_ciphertext"
pub fn encrypt(plaintext: &str, passphrase: &str) -> Result<String, EncryptionError> {
    let encryptor = age::Encryptor::with_user_passphrase(age::secrecy::SecretString::from(passphrase.to_string()));

    let mut encrypted = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted)?;
    writer.write_all(plaintext.as_bytes())?;
    writer.finish()?;

    let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &encrypted);
    Ok(format!("{}{}", AGE_PREFIX, encoded))
}

/// Decrypt a secret value with a passphrase using AGE
///
/// Expects a string in the format "age:base64_encoded_ciphertext"
pub fn decrypt(encrypted_value: &str, passphrase: &str) -> Result<String, EncryptionError> {
    let encoded = encrypted_value
        .strip_prefix(AGE_PREFIX)
        .ok_or(EncryptionError::InvalidFormat)?;

    let encrypted = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
        .map_err(|_| EncryptionError::InvalidBase64)?;

    let decryptor = match age::Decryptor::new(&encrypted[..])? {
        age::Decryptor::Passphrase(d) => d,
        _ => return Err(EncryptionError::UnsupportedRecipient),
    };

    let mut decrypted = vec![];
    let mut reader = decryptor.decrypt(
        &age::secrecy::SecretString::from(passphrase.to_string()),
        None,
    )?;
    reader.read_to_end(&mut decrypted)?;

    String::from_utf8(decrypted).map_err(|_| EncryptionError::InvalidUtf8)
}

/// Decrypt a value if it's encrypted, otherwise return as-is
///
/// This allows transparent handling of both encrypted and plain values.
pub fn decrypt_if_encrypted(value: &str, passphrase: Option<&str>) -> Result<String, EncryptionError> {
    if is_encrypted(value) {
        let pass = passphrase.ok_or(EncryptionError::NoPassphrase)?;
        decrypt(value, pass)
    } else {
        Ok(value.to_string())
    }
}

/// Encryption-related errors
#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    #[error("Invalid encrypted value format (expected 'age:...')")]
    InvalidFormat,

    #[error("Invalid base64 encoding")]
    InvalidBase64,

    #[error("AGE encryption error: {0}")]
    Age(#[from] age::EncryptError),

    #[error("AGE decryption error: {0}")]
    AgeDecrypt(#[from] age::DecryptError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unsupported recipient type (only passphrase encryption is supported)")]
    UnsupportedRecipient,

    #[error("Decrypted value is not valid UTF-8")]
    InvalidUtf8,

    #[error("No passphrase provided for decryption")]
    NoPassphrase,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let plaintext = "my-secret-api-key";
        let passphrase = "test-passphrase-123";

        let encrypted = encrypt(plaintext, passphrase).unwrap();
        assert!(encrypted.starts_with(AGE_PREFIX));
        assert!(is_encrypted(&encrypted));

        let decrypted = decrypt(&encrypted, passphrase).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_wrong_passphrase() {
        let plaintext = "my-secret-api-key";
        let passphrase = "correct-passphrase";
        let wrong_passphrase = "wrong-passphrase";

        let encrypted = encrypt(plaintext, passphrase).unwrap();
        let result = decrypt(&encrypted, wrong_passphrase);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_encrypted() {
        assert!(is_encrypted("age:YWdlLWVuY3J5cHRpb24u..."));
        assert!(!is_encrypted("sk-12345"));
        assert!(!is_encrypted("plain-value"));
    }

    #[test]
    fn test_decrypt_if_encrypted_plain() {
        let plain = "sk-12345";
        let result = decrypt_if_encrypted(plain, None).unwrap();
        assert_eq!(result, plain);
    }

    #[test]
    fn test_decrypt_if_encrypted_encrypted() {
        let plaintext = "sk-12345";
        let passphrase = "test-pass";

        let encrypted = encrypt(plaintext, passphrase).unwrap();
        let result = decrypt_if_encrypted(&encrypted, Some(passphrase)).unwrap();
        assert_eq!(result, plaintext);
    }

    #[test]
    fn test_decrypt_if_encrypted_no_passphrase() {
        let plaintext = "sk-12345";
        let passphrase = "test-pass";

        let encrypted = encrypt(plaintext, passphrase).unwrap();
        let result = decrypt_if_encrypted(&encrypted, None);
        assert!(matches!(result, Err(EncryptionError::NoPassphrase)));
    }
}
