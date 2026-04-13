//! Encryption/decryption command handlers

use xls_rs::encryption::{DataEncryptor, EncryptionAlgorithm};
use anyhow::Result;

/// Handle the encrypt command
///
/// Encrypts a file using the specified algorithm.
pub fn handle_encrypt(
    input: String,
    output: String,
    algorithm: String,
    key_file: Option<String>,
) -> Result<()> {
    let algorithm = match algorithm.to_lowercase().as_str() {
        "aes" | "aes256" => EncryptionAlgorithm::Aes256,
        "xor" => EncryptionAlgorithm::Xor,
        _ => anyhow::bail!("Unknown encryption algorithm: {}", algorithm),
    };

    let encryptor = DataEncryptor::new(algorithm);
    let key = get_encryption_key(key_file)?;

    // Validate input/output paths for security
    validate_file_path(&input)?;
    validate_file_path(&output)?;

    encryptor.encrypt_file(&input, &output, &key)?;

    println!("Encrypted {} to {} using {:?}", input, output, algorithm);

    Ok(())
}

/// Handle the decrypt command
///
/// Decrypts a file.
pub fn handle_decrypt(input: String, output: String, key_file: Option<String>) -> Result<()> {
    let encryptor = DataEncryptor::new(EncryptionAlgorithm::Aes256);
    let key = get_encryption_key(key_file)?;

    // Validate input/output paths for security
    validate_file_path(&input)?;
    validate_file_path(&output)?;

    encryptor.decrypt_file(&input, &output, &key)?;

    println!("Decrypted {} to {}", input, output);

    Ok(())
}

/// Get encryption key from file or environment variable
///
/// Security: Requires explicit key source - no hardcoded defaults
fn get_encryption_key(key_file: Option<String>) -> Result<Vec<u8>> {
    // First try key file parameter
    if let Some(key_path) = key_file {
        let encryptor = DataEncryptor::new(EncryptionAlgorithm::Aes256);
        return encryptor.load_key_from_file(&key_path);
    }

    // Then try environment variable
    if let Ok(key_str) = std::env::var("XLS_RS_ENCRYPTION_KEY") {
        if key_str.len() < 16 {
            anyhow::bail!("Encryption key from environment must be at least 16 bytes");
        }
        return Ok(key_str.into_bytes());
    }

    anyhow::bail!(
        "No encryption key provided. Use --key-file or set XLS_RS_ENCRYPTION_KEY environment variable"
    );
}

/// Validate file path for security
fn validate_file_path(path: &str) -> Result<()> {
    // Basic security checks
    if path.contains("..") {
        anyhow::bail!("Path traversal detected in: {}", path);
    }

    if path.starts_with('/') && !path.starts_with("/tmp/") && !path.starts_with("/var/tmp/") {
        anyhow::bail!("Absolute paths restricted to /tmp and /var/tmp: {}", path);
    }

    Ok(())
}
