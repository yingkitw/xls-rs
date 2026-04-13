//! Tests for encryption module

use xls_rs::encryption::{DataEncryptor, EncryptionAlgorithm};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_xor_encrypt_decrypt_data() {
    let encryptor = DataEncryptor::new(EncryptionAlgorithm::Xor);
    let key = b"test-key-123";
    let data = b"Hello, World!";

    let encrypted = encryptor.encrypt_data(data, key).unwrap();
    assert_ne!(encrypted, data);

    let decrypted = encryptor.decrypt_data(&encrypted, key).unwrap();
    assert_eq!(decrypted, data);
}

#[test]
fn test_aes256_encrypt_decrypt_data() {
    let encryptor = DataEncryptor::new(EncryptionAlgorithm::Aes256);
    let key = b"test-key-123";
    let data = b"Sensitive data";

    let encrypted = encryptor.encrypt_data(data, key).unwrap();
    assert_ne!(encrypted, data);

    let decrypted = encryptor.decrypt_data(&encrypted, key).unwrap();
    assert_eq!(decrypted, data);
}

#[test]
fn test_encrypt_decrypt_file() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("input.txt");
    let encrypted_path = dir.path().join("encrypted.bin");
    let decrypted_path = dir.path().join("decrypted.txt");

    let original_data = "This is test data for encryption";
    fs::write(&input_path, original_data).unwrap();

    let encryptor = DataEncryptor::new(EncryptionAlgorithm::Xor);
    let key = b"secret-key-456";

    encryptor
        .encrypt_file(
            input_path.to_str().unwrap(),
            encrypted_path.to_str().unwrap(),
            key,
        )
        .unwrap();

    assert!(encrypted_path.exists());
    let encrypted_content = fs::read(&encrypted_path).unwrap();
    assert_ne!(encrypted_content, original_data.as_bytes());

    encryptor
        .decrypt_file(
            encrypted_path.to_str().unwrap(),
            decrypted_path.to_str().unwrap(),
            key,
        )
        .unwrap();

    let decrypted_content = fs::read_to_string(&decrypted_path).unwrap();
    assert_eq!(decrypted_content, original_data);
}

#[test]
fn test_empty_key_error() {
    let encryptor = DataEncryptor::new(EncryptionAlgorithm::Xor);
    let data = b"test data";
    let empty_key = b"";

    let result = encryptor.encrypt_data(data, empty_key);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot be empty"));
}

#[test]
fn test_encrypt_empty_data() {
    let encryptor = DataEncryptor::new(EncryptionAlgorithm::Xor);
    let key = b"test-key";
    let empty_data = b"";

    let encrypted = encryptor.encrypt_data(empty_data, key).unwrap();
    assert_eq!(encrypted.len(), 0);

    let decrypted = encryptor.decrypt_data(&encrypted, key).unwrap();
    assert_eq!(decrypted, empty_data);
}

#[test]
fn test_encrypt_large_data() {
    let encryptor = DataEncryptor::new(EncryptionAlgorithm::Xor);
    let key = b"test-key";
    let large_data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();

    let encrypted = encryptor.encrypt_data(&large_data, key).unwrap();
    assert_eq!(encrypted.len(), large_data.len());

    let decrypted = encryptor.decrypt_data(&encrypted, key).unwrap();
    assert_eq!(decrypted, large_data);
}

#[test]
fn test_different_keys_produce_different_results() {
    let encryptor = DataEncryptor::new(EncryptionAlgorithm::Xor);
    let data = b"test data";
    let key1 = b"key1";
    let key2 = b"key2";

    let encrypted1 = encryptor.encrypt_data(data, key1).unwrap();
    let encrypted2 = encryptor.encrypt_data(data, key2).unwrap();

    assert_ne!(encrypted1, encrypted2);
}

#[test]
fn test_load_key_from_file() {
    let dir = tempdir().unwrap();
    let key_path = dir.path().join("key.bin");
    let key_data = b"my-secret-key-12345";

    fs::write(&key_path, key_data).unwrap();

    let encryptor = DataEncryptor::new(EncryptionAlgorithm::Xor);
    let loaded_key = encryptor.load_key_from_file(key_path.to_str().unwrap()).unwrap();

    assert_eq!(loaded_key, key_data);
}

#[test]
fn test_load_key_from_nonexistent_file() {
    let encryptor = DataEncryptor::new(EncryptionAlgorithm::Xor);
    let result = encryptor.load_key_from_file("/nonexistent/key.bin");

    assert!(result.is_err());
}
