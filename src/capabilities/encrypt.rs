//! Encrypt capability

use crate::capabilities::{Capability, CapabilityMetadata};
use crate::encryption::{DataEncryptor, EncryptionAlgorithm};
use anyhow::{Context, Result};
use serde_json::{json, Value};

pub struct EncryptCapability;

impl Capability for EncryptCapability {
    fn metadata(&self) -> CapabilityMetadata {
        CapabilityMetadata {
            name: "encrypt".to_string(),
            description: "Encrypt a file using AES-256 or XOR".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string", "description": "Input file path" },
                    "output": { "type": "string", "description": "Output encrypted file path" },
                    "algorithm": { "type": "string", "description": "Encryption algorithm: aes256 or xor (default: xor)" },
                    "key": { "type": "string", "description": "Encryption key string" },
                    "key_file": { "type": "string", "description": "Path to file containing encryption key" }
                },
                "required": ["input", "output"]
            }),
        }
    }

    fn execute(&self, args: Value) -> Result<Value> {
        let input = args["input"].as_str().context("Missing input")?;
        let output = args["output"].as_str().context("Missing output")?;
        let algorithm = args["algorithm"].as_str().unwrap_or("xor");
        let key_str = args["key"].as_str();
        let key_file = args["key_file"].as_str();

        let algorithm = match algorithm.to_lowercase().as_str() {
            "aes256" => EncryptionAlgorithm::Aes256,
            "xor" => EncryptionAlgorithm::Xor,
            _ => anyhow::bail!("Unknown algorithm: {} (use aes256 or xor)", algorithm),
        };

        let encryptor = DataEncryptor::new(algorithm);
        let key = if let Some(path) = key_file {
            encryptor.load_key_from_file(path)?
        } else if let Some(k) = key_str {
            k.as_bytes().to_vec()
        } else {
            anyhow::bail!("Either 'key' or 'key_file' must be provided")
        };

        encryptor.encrypt_file(input, output, &key)?;

        Ok(json!({
            "status": "success",
            "input": input,
            "output": output,
            "algorithm": format!("{:?}", algorithm),
        }))
    }
}
