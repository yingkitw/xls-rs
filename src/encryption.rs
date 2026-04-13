//! Data encryption operations
//!
//! Provides encryption and decryption capabilities for data files.

use anyhow::Result;
use std::fs;
use std::io::{Read, Write};

/// Encryption algorithm
#[derive(Debug, Clone, Copy)]
pub enum EncryptionAlgorithm {
    Aes256,
    Xor, // Simple XOR for testing (not secure for production)
}

/// Data encryptor/decryptor
pub struct DataEncryptor {
    algorithm: EncryptionAlgorithm,
}

impl DataEncryptor {
    pub fn new(algorithm: EncryptionAlgorithm) -> Self {
        Self { algorithm }
    }

    /// Encrypt a file
    pub fn encrypt_file(&self, input_path: &str, output_path: &str, key: &[u8]) -> Result<()> {
        let mut input_data = Vec::new();
        let mut file = std::fs::File::open(input_path)
            .with_context(|| format!("Failed to open input file: {}", input_path))?;
        file.read_to_end(&mut input_data)?;

        let encrypted = match self.algorithm {
            EncryptionAlgorithm::Aes256 => {
                // For now, use simple XOR as placeholder
                // In production, use proper AES-256-GCM
                self.xor_encrypt(&input_data, key)
            }
            EncryptionAlgorithm::Xor => self.xor_encrypt(&input_data, key),
        }?;

        let mut output_file = std::fs::File::create(output_path)
            .with_context(|| format!("Failed to create output file: {}", output_path))?;
        output_file.write_all(&encrypted)?;

        Ok(())
    }

    /// Decrypt a file
    pub fn decrypt_file(&self, input_path: &str, output_path: &str, key: &[u8]) -> Result<()> {
        // XOR encryption is symmetric
        self.encrypt_file(input_path, output_path, key)
    }

    /// Encrypt data in memory
    pub fn encrypt_data(&self, data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        match self.algorithm {
            EncryptionAlgorithm::Aes256 => self.xor_encrypt(data, key),
            EncryptionAlgorithm::Xor => self.xor_encrypt(data, key),
        }
    }

    /// Decrypt data in memory
    pub fn decrypt_data(&self, data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        // XOR is symmetric
        self.encrypt_data(data, key)
    }

    fn xor_encrypt(&self, data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        if key.is_empty() {
            anyhow::bail!("Encryption key cannot be empty");
        }

        Ok(data
            .iter()
            .enumerate()
            .map(|(i, &byte)| byte ^ key[i % key.len()])
            .collect())
    }

    /// Load key from file
    pub fn load_key_from_file(&self, key_path: &str) -> Result<Vec<u8>> {
        fs::read(key_path).with_context(|| format!("Failed to read key file: {}", key_path))
    }
}

use anyhow::Context;
