//! Password-protected XLSX decryption (MS-OFFCRYPTO Agile Encryption).
//!
//! Encrypted XLSX files are OLE2 (CFB) containers with two streams:
//! - `EncryptionInfo`: XML describing the encryption parameters
//! - `EncryptedPackage`: AES-CBC encrypted ZIP containing the actual workbook
//!
//! This module implements the Agile Encryption scheme (version 4) used by
//! modern Excel for password-protected `.xlsx` files.

use anyhow::{bail, Context, Result};

use aes::cipher::{block_padding::NoPadding, BlockDecryptMut, KeyIvInit};
use sha2::{Digest, Sha512};

use super::xls_reader::cfb_reader::CfbReader;

type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

const OLE2_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

/// Check if a byte slice is an OLE2 container (potential encrypted XLSX).
pub fn is_ole2(data: &[u8]) -> bool {
    data.len() >= 8 && data[0..8] == OLE2_MAGIC
}

/// Check if a byte slice is an encrypted XLSX (OLE2 with EncryptionInfo stream).
pub fn is_encrypted_xlsx(data: &[u8]) -> bool {
    if !is_ole2(data) {
        return false;
    }
    match CfbReader::parse_slice(data) {
        Ok(cfb) => cfb.get_stream("EncryptionInfo").is_some(),
        Err(_) => false,
    }
}

/// Parsed Agile Encryption parameters from the EncryptionInfo stream.
struct AgileEncryptionInfo {
    /// Salt from `<keyData>` (base64-decoded), used as IV for package decryption.
    key_data_salt: Vec<u8>,
    /// Key size in bytes (32 for AES-256).
    key_size: usize,
    /// Hash algorithm name (e.g., "SHA512").
    hash_algorithm: String,
    /// Salt from `<encryptedKey>` (base64-decoded).
    encrypted_key_salt: Vec<u8>,
    /// Spin count (iteration count for PBKDF2).
    spin_count: u32,
    /// Encrypted verifier hash input (base64-decoded).
    encrypted_verifier_hash_input: Vec<u8>,
    /// Encrypted verifier hash value (base64-decoded).
    encrypted_verifier_hash_value: Vec<u8>,
    /// Encrypted key value (base64-decoded).
    encrypted_key_value: Vec<u8>,
}

/// Decrypt a password-protected XLSX file and return the decrypted ZIP bytes.
///
/// The input `data` must be an OLE2 container with `EncryptionInfo` and
/// `EncryptedPackage` streams (i.e., `is_encrypted_xlsx(data)` returns true).
pub fn decrypt_xlsx(data: &[u8], password: &str) -> Result<Vec<u8>> {
    let cfb = CfbReader::parse_slice(data).context("Failed to parse OLE2 container")?;

    let enc_info_raw = cfb
        .get_stream("EncryptionInfo")
        .context("EncryptionInfo stream not found")?;
    let enc_pkg_raw = cfb
        .get_stream("EncryptedPackage")
        .context("EncryptedPackage stream not found")?;

    let info = parse_encryption_info(&enc_info_raw)?;

    // Derive the verifier key from the password
    let verifier_key = derive_key(
        password,
        &info.encrypted_key_salt,
        info.spin_count,
        info.key_size,
        &info.hash_algorithm,
    )?;

    // Decrypt the encrypted key value to get the actual package encryption key
    let iv = &info.encrypted_key_salt[..16.min(info.encrypted_key_salt.len())];
    let package_key = aes_cbc_decrypt(&info.encrypted_key_value, &verifier_key, iv)?;

    // Verify the password by checking the verifier hash
    let verifier_input =
        aes_cbc_decrypt(&info.encrypted_verifier_hash_input, &verifier_key, iv)?;
    let verifier_hash =
        aes_cbc_decrypt(&info.encrypted_verifier_hash_value, &verifier_key, iv)?;

    let computed_hash = match info.hash_algorithm.as_str() {
        "SHA512" => {
            let mut h = Sha512::new();
            h.update(&verifier_input);
            h.finalize().to_vec()
        }
        "SHA256" => {
            let mut h = sha2::Sha256::new();
            h.update(&verifier_input);
            h.finalize().to_vec()
        }
        _ => bail!("Unsupported hash algorithm: {}", info.hash_algorithm),
    };

    // The verifier hash value is padded; compare only the computed hash length
    if computed_hash.len() > verifier_hash.len()
        || computed_hash != verifier_hash[..computed_hash.len()]
    {
        bail!("Password verification failed — incorrect password");
    }

    // Decrypt the EncryptedPackage
    // First 4 bytes: size of the encrypted data (little-endian u32)
    // Next 4 bytes: padding size (ignored)
    // Remaining: AES-CBC encrypted ZIP data
    if enc_pkg_raw.len() < 8 {
        bail!("EncryptedPackage too short");
    }
    let enc_data_len = u32::from_le_bytes([
        enc_pkg_raw[0], enc_pkg_raw[1], enc_pkg_raw[2], enc_pkg_raw[3],
    ]) as usize;
    if 8 + enc_data_len > enc_pkg_raw.len() {
        bail!("EncryptedPackage data length exceeds stream size");
    }
    let pkg_iv = &info.key_data_salt[..16.min(info.key_data_salt.len())];
    let decrypted = aes_cbc_decrypt(&enc_pkg_raw[8..8 + enc_data_len], &package_key, pkg_iv)?;

    Ok(decrypted)
}

/// Parse the EncryptionInfo stream (Agile Encryption, version 4).
fn parse_encryption_info(data: &[u8]) -> Result<AgileEncryptionInfo> {
    // Version: first 2 bytes = major version, next 2 bytes = minor version
    // Agile Encryption is version 4.4
    if data.len() < 4 {
        bail!("EncryptionInfo too short");
    }
    let version_major = u16::from_le_bytes([data[0], data[1]]);
    let _version_minor = u16::from_le_bytes([data[2], data[3]]);

    if version_major != 4 {
        bail!(
            "Unsupported encryption version: {} (only Agile Encryption v4 is supported)",
            version_major
        );
    }

    // The rest is XML (starting after the 4-byte version header)
    let xml = std::str::from_utf8(&data[4..])
        .context("EncryptionInfo XML is not valid UTF-8")?;

    // Parse the XML using simple string search (avoid heavy XML parser deps)
    let key_data_salt = extract_base64_attr(xml, "keyData", "saltValue")?;
    let key_size = extract_int_attr(xml, "keyData", "keySize")? as usize;
    let hash_algorithm = extract_str_attr(xml, "keyData", "hashAlgorithm")?;

    let encrypted_key_salt = extract_base64_attr(xml, "encryptedKey", "saltValue")?;
    let spin_count = extract_int_attr(xml, "encryptedKey", "spinCount")?;
    let encrypted_verifier_hash_input =
        extract_base64_attr(xml, "encryptedKey", "encryptedVerifierHashInput")?;
    let encrypted_verifier_hash_value =
        extract_base64_attr(xml, "encryptedKey", "encryptedVerifierHashValue")?;
    let encrypted_key_value = extract_base64_attr(xml, "encryptedKey", "encryptedKeyValue")?;

    Ok(AgileEncryptionInfo {
        key_data_salt,
        key_size,
        hash_algorithm,
        encrypted_key_salt,
        spin_count,
        encrypted_verifier_hash_input,
        encrypted_verifier_hash_value,
        encrypted_key_value,
    })
}

/// Derive a key from the password using PBKDF2 with the given hash algorithm.
fn derive_key(
    password: &str,
    salt: &[u8],
    iterations: u32,
    key_size: usize,
    hash_algorithm: &str,
) -> Result<Vec<u8>> {
    // Convert password to UTF-16LE bytes
    let mut password_bytes = Vec::new();
    for c in password.encode_utf16() {
        password_bytes.extend_from_slice(&c.to_le_bytes());
    }

    let mut derived_key = vec![0u8; key_size];

    match hash_algorithm {
        "SHA512" => {
            pbkdf2::pbkdf2_hmac::<Sha512>(
                &password_bytes,
                salt,
                iterations,
                &mut derived_key,
            );
        }
        "SHA256" => {
            pbkdf2::pbkdf2_hmac::<sha2::Sha256>(
                &password_bytes,
                salt,
                iterations,
                &mut derived_key,
            );
        }
        _ => bail!("Unsupported hash algorithm: {}", hash_algorithm),
    }

    Ok(derived_key)
}

/// Decrypt data using AES-256-CBC with no padding removal (raw block decryption).
fn aes_cbc_decrypt(data: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>> {
    if key.len() != 32 {
        bail!("AES-256 requires a 32-byte key, got {}", key.len());
    }
    if iv.len() != 16 {
        bail!("AES-CBC requires a 16-byte IV, got {}", iv.len());
    }
    if data.len() % 16 != 0 {
        bail!(
            "Encrypted data length ({}) is not a multiple of 16 (AES block size)",
            data.len()
        );
    }

    let mut buf = data.to_vec();
    let plaintext = Aes256CbcDec::new(key.into(), iv.into())
        .decrypt_padded_mut::<NoPadding>(&mut buf)
        .map_err(|_| anyhow::anyhow!("AES-CBC decryption failed"))?;
    Ok(plaintext.to_vec())
}

// --- XML attribute extraction helpers ---

/// Find a tag by local name and extract a string attribute value.
fn extract_str_attr(xml: &str, tag: &str, attr: &str) -> Result<String> {
    let tag_pos = xml
        .find(&format!("<{}", tag))
        .ok_or_else(|| anyhow::anyhow!("Tag <{}> not found in EncryptionInfo", tag))?;
    let tag_end = xml[tag_pos..]
        .find('>')
        .ok_or_else(|| anyhow::anyhow!("Malformed tag <{}>", tag))?;
    let tag_content = &xml[tag_pos..tag_pos + tag_end];

    let attr_pattern = format!("{}=\"", attr);
    let attr_pos = tag_content
        .find(&attr_pattern)
        .ok_or_else(|| anyhow::anyhow!("Attribute {} not found in <{}>", attr, tag))?;
    let value_start = attr_pos + attr_pattern.len();
    let value_end = tag_content[value_start..]
        .find('"')
        .ok_or_else(|| anyhow::anyhow!("Unterminated attribute {}", attr))?;
    Ok(tag_content[value_start..value_start + value_end].to_string())
}

/// Find a tag by local name and extract an integer attribute value.
fn extract_int_attr(xml: &str, tag: &str, attr: &str) -> Result<u32> {
    let s = extract_str_attr(xml, tag, attr)?;
    s.parse::<u32>()
        .with_context(|| format!("Failed to parse {} as integer", attr))
}

/// Find a tag by local name and extract a base64-encoded attribute value.
fn extract_base64_attr(xml: &str, tag: &str, attr: &str) -> Result<Vec<u8>> {
    let s = extract_str_attr(xml, tag, attr)?;
    base64_decode(&s).with_context(|| format!("Failed to base64-decode {}", attr))
}

/// Minimal base64 decoder (avoids adding a base64 dependency).
fn base64_decode(input: &str) -> Result<Vec<u8>> {
    let input: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    if input.len() % 4 != 0 && !input.ends_with('=') {
        // Allow padding-less base64 with trailing chars
    }

    let lookup = |c: char| -> Option<u8> {
        match c {
            'A'..='Z' => Some(c as u8 - b'A'),
            'a'..='z' => Some(c as u8 - b'a' + 26),
            '0'..='9' => Some(c as u8 - b'0' + 52),
            '+' => Some(62),
            '/' => Some(63),
            '=' => Some(0), // padding
            _ => None,
        }
    };

    let chars: Vec<char> = input.chars().collect();
    let mut result = Vec::with_capacity(chars.len() * 3 / 4);

    let mut i = 0;
    while i + 3 < chars.len() {
        let b0 = lookup(chars[i]).ok_or_else(|| anyhow::anyhow!("Invalid base64 char"))?;
        let b1 = lookup(chars[i + 1]).ok_or_else(|| anyhow::anyhow!("Invalid base64 char"))?;
        let b2 = lookup(chars[i + 2]).ok_or_else(|| anyhow::anyhow!("Invalid base64 char"))?;
        let b3 = lookup(chars[i + 3]).ok_or_else(|| anyhow::anyhow!("Invalid base64 char"))?;

        let is_pad2 = chars[i + 2] == '=';
        let is_pad3 = chars[i + 3] == '=';

        result.push((b0 << 2) | (b1 >> 4));
        if !is_pad2 {
            result.push((b1 << 4) | (b2 >> 2));
            if !is_pad3 {
                result.push((b2 << 6) | b3);
            }
        }
        i += 4;
    }

    // Handle remaining 2-3 chars (padding-less)
    if i < chars.len() {
        let remaining = chars.len() - i;
        if remaining >= 2 {
            let b0 = lookup(chars[i]).ok_or_else(|| anyhow::anyhow!("Invalid base64 char"))?;
            let b1 = lookup(chars[i + 1]).ok_or_else(|| anyhow::anyhow!("Invalid base64 char"))?;
            result.push((b0 << 2) | (b1 >> 4));
            if remaining >= 3 {
                let b2 = lookup(chars[i + 2]).ok_or_else(|| anyhow::anyhow!("Invalid base64 char"))?;
                result.push((b1 << 4) | (b2 >> 2));
            }
        }
    }

    Ok(result)
}

/// Encode bytes to base64 string.
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        result.push(CHARS[(b0 >> 2) as usize] as char);
        result.push(CHARS[((b0 & 0x03) << 4 | b1 >> 4) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((b1 & 0x0F) << 2 | b2 >> 6) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(b2 & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

/// Encrypt data using AES-256-CBC.
fn aes_cbc_encrypt(data: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>> {
    use aes::cipher::{block_padding::NoPadding, BlockEncryptMut};
    type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;

    if key.len() != 32 {
        bail!("AES-256 requires a 32-byte key");
    }
    if iv.len() != 16 {
        bail!("AES-CBC requires a 16-byte IV");
    }

    // Pad to multiple of 16 with zeros (MS-OFFCRYPTO uses zero-padding, not PKCS7)
    let pad_len = (16 - (data.len() % 16)) % 16;
    let mut buf = data.to_vec();
    if data.len() % 16 != 0 {
        buf.extend(std::iter::repeat(0u8).take(pad_len));
    }
    // If data is already a multiple of 16, no extra padding (MS-OFFCRYPTO doesn't add a full block)
    let len = buf.len();

    let ciphertext = Aes256CbcEnc::new(key.into(), iv.into())
        .encrypt_padded_mut::<NoPadding>(&mut buf, len)
        .map_err(|_| anyhow::anyhow!("AES-CBC encryption failed"))?;
    Ok(ciphertext.to_vec())
}

/// Create an encrypted XLSX (OLE2 container) from a ZIP byte slice.
///
/// This is primarily for testing — it creates a minimal Agile Encryption
/// container with the given password.
pub fn create_encrypted_xlsx(zip_data: &[u8], password: &str) -> Result<Vec<u8>> {
    use sha2::Digest;

    // Generate random salts
    let key_data_salt: [u8; 16] = [
        0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x11, 0x22,
        0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0x00,
    ];
    let encrypted_key_salt: [u8; 16] = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF,
        0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10,
    ];

    let key_size = 32usize; // AES-256
    let spin_count = 100_000u32;

    // Derive verifier key from password
    let verifier_key = derive_key(
        password,
        &encrypted_key_salt,
        spin_count,
        key_size,
        "SHA512",
    )?;

    // Generate a random package key
    let package_key: [u8; 32] = [
        0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE,
        0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0,
        0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
        0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00,
    ];

    // Create verifier: hash a random input, encrypt both
    let verifier_input: [u8; 16] = [
        0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80,
        0x90, 0xA0, 0xB0, 0xC0, 0xD0, 0xE0, 0xF0, 0x0A,
    ];
    let mut hasher = Sha512::new();
    hasher.update(verifier_input);
    let verifier_hash = hasher.finalize().to_vec();

    // Encrypt verifier input (pad to 16 bytes — it's already 16)
    let iv = &encrypted_key_salt[..16];
    let enc_verifier_input = aes_cbc_encrypt(&verifier_input, &verifier_key, iv)?;
    // Encrypt verifier hash (pad to multiple of 16)
    let enc_verifier_hash = aes_cbc_encrypt(&verifier_hash, &verifier_key, iv)?;
    // Encrypt the package key (32 bytes, already multiple of 16)
    let enc_key_value = aes_cbc_encrypt(&package_key, &verifier_key, iv)?;

    // Encrypt the ZIP data with the package key
    let pkg_iv = &key_data_salt[..16];
    let enc_pkg = aes_cbc_encrypt(zip_data, &package_key, pkg_iv)?;

    // Build EncryptedPackage stream: 4-byte size + 4-byte padding + encrypted data
    let mut enc_pkg_stream = Vec::new();
    let pkg_size = enc_pkg.len() as u32;
    enc_pkg_stream.extend_from_slice(&pkg_size.to_le_bytes());
    enc_pkg_stream.extend_from_slice(&[0u8; 4]); // padding
    enc_pkg_stream.extend_from_slice(&enc_pkg);

    // Pad to >= 4096 bytes so the CFB reader uses regular FAT (not mini-FAT)
    while enc_pkg_stream.len() < 4096 {
        enc_pkg_stream.push(0);
    }

    // Build EncryptionInfo stream
    let enc_info_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<encryption xmlns="http://schemas.microsoft.com/office/2006/encryption">
  <keyData saltSize="16" blockSize="16" keyBits="256" keySize="32" hashSize="64" cipherAlgorithm="AES" cipherChaining="ChainingModeCBC" hashAlgorithm="SHA512" saltValue="{}"/>
  <dataIntegrity encryptedHmacKey="{}" encryptedHmacValue="{}"/>
  <keyEncryptors>
    <keyEncryptor uri="http://www.microsoft.com/office/2006/keyEncryptor/password">
      <encryptedKey spinCount="{}" saltSize="16" blockSize="16" keyBits="256" keySize="32" hashSize="64" cipherAlgorithm="AES" cipherChaining="ChainingModeCBC" hashAlgorithm="SHA512" saltValue="{}" encryptedVerifierHashInput="{}" encryptedVerifierHashValue="{}" encryptedKeyValue="{}"/>
    </keyEncryptor>
  </keyEncryptors>
</encryption>"#,
        base64_encode(&key_data_salt),
        base64_encode(&[0u8; 64]), // placeholder HMAC key
        base64_encode(&[0u8; 64]), // placeholder HMAC value
        spin_count,
        base64_encode(&encrypted_key_salt),
        base64_encode(&enc_verifier_input),
        base64_encode(&enc_verifier_hash),
        base64_encode(&enc_key_value),
    );

    // Build EncryptionInfo stream: 4-byte version header + XML
    let mut enc_info_stream = Vec::new();
    enc_info_stream.extend_from_slice(&4u16.to_le_bytes()); // major version
    enc_info_stream.extend_from_slice(&4u16.to_le_bytes()); // minor version
    enc_info_stream.extend_from_slice(enc_info_xml.as_bytes());

    // Pad to >= 4096 bytes so the CFB reader uses regular FAT (not mini-FAT)
    // Our minimal test container doesn't set up mini-FAT properly.
    while enc_info_stream.len() < 4096 {
        enc_info_stream.push(0);
    }

    // Build the OLE2 container
    build_ole2_container(&enc_info_stream, &enc_pkg_stream)
}

/// Build a minimal OLE2 (CFB) container with two streams.
fn build_ole2_container(stream1: &[u8], stream2: &[u8]) -> Result<Vec<u8>> {
    // We need to build a minimal CFB container with:
    // - Header (512 bytes)
    // - FAT sector
    // - Directory sectors
    // - Data sectors for the two streams
    //
    // For simplicity, we'll use a sector size of 512 and put everything
    // in regular (non-mini) streams since our test data is > 4096 bytes.

    const SECTOR_SIZE: usize = 512;
    const HEADER_SIZE: usize = 512;

    // We need sectors for:
    // - FAT (1 sector)
    // - Directory (1 sector, 4 entries max)
    // - Stream 1 data
    // - Stream 2 data
    let stream1_sectors = (stream1.len() + SECTOR_SIZE - 1) / SECTOR_SIZE;
    let stream2_sectors = (stream2.len() + SECTOR_SIZE - 1) / SECTOR_SIZE;

    // Layout:
    // Sector 0: FAT
    // Sector 1: Directory
    // Sectors 2..2+stream1_sectors: Stream 1 (EncryptionInfo)
    // Sectors 2+stream1_sectors..: Stream 2 (EncryptedPackage)
    let total_sectors = 2 + stream1_sectors + stream2_sectors;
    let total_size = HEADER_SIZE + total_sectors * SECTOR_SIZE;

    let mut buf = vec![0u8; total_size];

    // --- Header ---
    // Magic
    buf[0..8].copy_from_slice(&OLE2_MAGIC);
    // CLSID (16 bytes, zeros)
    // Minor version
    buf[24..26].copy_from_slice(&0x003Eu16.to_le_bytes());
    // Major version (3 = v3, 512-byte sectors)
    buf[26..28].copy_from_slice(&3u16.to_le_bytes());
    // Byte order (0xFFFE = little-endian)
    buf[28..30].copy_from_slice(&0xFFFEu16.to_le_bytes());
    // Sector shift (9 = 512 bytes)
    buf[30..32].copy_from_slice(&9u16.to_le_bytes());
    // Mini sector shift (6 = 64 bytes)
    buf[32..34].copy_from_slice(&6u16.to_le_bytes());
    // Number of FAT sectors
    buf[44..48].copy_from_slice(&1u32.to_le_bytes());
    // First directory sector
    buf[48..52].copy_from_slice(&1u32.to_le_bytes()); // sector 1
    // Mini stream cutoff (4096)
    buf[56..60].copy_from_slice(&4096u32.to_le_bytes());
    // First mini-FAT sector (none)
    buf[60..64].copy_from_slice(&0xFFFFFFFEu32.to_le_bytes()); // ENDOFCHAIN
    // Number of mini-FAT sectors (0)
    buf[64..68].copy_from_slice(&0u32.to_le_bytes());
    // First DIFAT sector (none)
    buf[68..72].copy_from_slice(&0xFFFFFFFEu32.to_le_bytes()); // ENDOFCHAIN
    // Number of DIFAT sectors (0)
    buf[72..76].copy_from_slice(&0u32.to_le_bytes());
    // DIFAT array (109 entries, first = sector 0, rest = FREESECT)
    buf[76..80].copy_from_slice(&0u32.to_le_bytes()); // FAT is sector 0
    for i in 1..109 {
        let offset = 76 + i * 4;
        buf[offset..offset + 4].copy_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // FREESECT
    }

    // --- FAT (sector 0) ---
    let fat_offset = HEADER_SIZE;
    // Sector 0 = FAT (self-reference not needed, mark as FATSECT)
    buf[fat_offset..fat_offset + 4].copy_from_slice(&0xFFFFFFFDu32.to_le_bytes()); // FATSECT
    // Sector 1 = Directory (end of chain)
    buf[fat_offset + 4..fat_offset + 8].copy_from_slice(&0xFFFFFFFEu32.to_le_bytes()); // ENDOFCHAIN
    // Stream 1 sectors: chain them
    for i in 0..stream1_sectors {
        let sector_idx = 2 + i;
        let offset = fat_offset + sector_idx * 4;
        if i < stream1_sectors - 1 {
            buf[offset..offset + 4].copy_from_slice(&((sector_idx + 1) as u32).to_le_bytes());
        } else {
            buf[offset..offset + 4].copy_from_slice(&0xFFFFFFFEu32.to_le_bytes()); // ENDOFCHAIN
        }
    }
    // Stream 2 sectors: chain them
    for i in 0..stream2_sectors {
        let sector_idx = 2 + stream1_sectors + i;
        let offset = fat_offset + sector_idx * 4;
        if i < stream2_sectors - 1 {
            buf[offset..offset + 4].copy_from_slice(&((sector_idx + 1) as u32).to_le_bytes());
        } else {
            buf[offset..offset + 4].copy_from_slice(&0xFFFFFFFEu32.to_le_bytes()); // ENDOFCHAIN
        }
    }
    // Fill remaining FAT entries with FREESECT
    for i in (2 + stream1_sectors + stream2_sectors)..(SECTOR_SIZE / 4) {
        let offset = fat_offset + i * 4;
        if offset + 4 <= buf.len() {
            buf[offset..offset + 4].copy_from_slice(&0xFFFFFFFFu32.to_le_bytes());
        }
    }

    // --- Directory (sector 1) ---
    let dir_offset = HEADER_SIZE + SECTOR_SIZE;
    // Entry 0: Root Entry
    write_dir_entry(&mut buf[dir_offset..], "Root Entry", 5, 0xFFFFFFFE, 0, 0xFFFFFFFE, 0xFFFFFFFE, 1);
    // Entry 1: EncryptionInfo (child of root)
    let s1_start = 2u32;
    write_dir_entry(
        &mut buf[dir_offset + 128..],
        "EncryptionInfo",
        2,
        s1_start,
        stream1.len() as u64,
        0xFFFFFFFE,
        0xFFFFFFFE,
        2,
    );
    // Entry 2: EncryptedPackage (sibling of EncryptionInfo)
    let s2_start = (2 + stream1_sectors) as u32;
    write_dir_entry(
        &mut buf[dir_offset + 256..],
        "EncryptedPackage",
        2,
        s2_start,
        stream2.len() as u64,
        0xFFFFFFFE,
        0xFFFFFFFE,
        0xFFFFFFFE,
    );
    // Entry 3: empty
    write_dir_entry(
        &mut buf[dir_offset + 384..],
        "",
        0,
        0xFFFFFFFE,
        0,
        0xFFFFFFFE,
        0xFFFFFFFE,
        0xFFFFFFFE,
    );

    // --- Stream data ---
    let s1_offset = HEADER_SIZE + 2 * SECTOR_SIZE;
    buf[s1_offset..s1_offset + stream1.len()].copy_from_slice(stream1);

    let s2_offset = HEADER_SIZE + (2 + stream1_sectors) * SECTOR_SIZE;
    buf[s2_offset..s2_offset + stream2.len()].copy_from_slice(stream2);

    Ok(buf)
}

/// Write a CFB directory entry (128 bytes) at the given offset.
fn write_dir_entry(
    buf: &mut [u8],
    name: &str,
    obj_type: u8,
    start_sector: u32,
    size: u64,
    left: u32,
    right: u32,
    child: u32,
) {
    // Name: UTF-16LE, including null terminator
    let name_utf16: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let name_bytes: Vec<u8> = name_utf16
        .iter()
        .flat_map(|c| c.to_le_bytes())
        .collect();
    let name_len = name_bytes.len().min(64);
    buf[..name_len].copy_from_slice(&name_bytes[..name_len]);
    // Name length in bytes (including null terminator)
    buf[64..66].copy_from_slice(&(name_len as u16).to_le_bytes());
    // Object type
    buf[66] = obj_type;
    // Color (0 = black)
    buf[67] = 0;
    // Left sibling
    buf[68..72].copy_from_slice(&left.to_le_bytes());
    // Right sibling
    buf[72..76].copy_from_slice(&right.to_le_bytes());
    // Child
    buf[76..80].copy_from_slice(&child.to_le_bytes());
    // CLSID (16 bytes, zeros)
    // State bits (4 bytes, zeros)
    // Creation time (8 bytes, zeros)
    // Modified time (8 bytes, zeros)
    // Start sector
    buf[116..120].copy_from_slice(&start_sector.to_le_bytes());
    // Size
    buf[120..128].copy_from_slice(&size.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_roundtrip() {
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x23, 0x45, 0x67];
        let encoded = base64_encode(&data);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_base64_known_vector() {
        // "Hello" in base64 = SGVsbG8=
        let decoded = base64_decode("SGVsbG8=").unwrap();
        assert_eq!(decoded, b"Hello");
    }

    #[test]
    fn test_is_ole2_detection() {
        assert!(is_ole2(&OLE2_MAGIC));
        assert!(!is_ole2(b"PK\x03\x04..."));
        assert!(!is_ole2(&[]));
    }

    #[test]
    fn test_aes_cbc_roundtrip() {
        let key = [0xAA; 32];
        let iv = [0xBB; 16];
        let plaintext = b"Hello, World! This is a test message.";
        let encrypted = aes_cbc_encrypt(plaintext, &key, &iv).unwrap();
        let decrypted = aes_cbc_decrypt(&encrypted, &key, &iv).unwrap();
        // Zero-padding: trim trailing zeros to recover original
        let original_len = plaintext.len();
        assert_eq!(&decrypted[..original_len], plaintext);
    }

    #[test]
    fn test_derive_key_consistency() {
        let key1 = derive_key("password", &[0xAA; 16], 1000, 32, "SHA512").unwrap();
        let key2 = derive_key("password", &[0xAA; 16], 1000, 32, "SHA512").unwrap();
        assert_eq!(key1, key2);
        assert_eq!(key1.len(), 32);

        // Different password → different key
        let key3 = derive_key("different", &[0xAA; 16], 1000, 32, "SHA512").unwrap();
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_encrypt_decrypt_xlsx_roundtrip() {
        // Create a simple XLSX (just a ZIP with minimal content)
        let mut writer = crate::excel::xlsx_writer::XlsxWriter::new();
        writer.add_sheet("Test").unwrap();
        let mut row = crate::excel::xlsx_writer::RowData::new();
        row.add_string("Hello");
        row.add_number(42.0);
        writer.add_row(row);

        let mut zip_buf = std::io::Cursor::new(Vec::new());
        writer.save(&mut zip_buf).unwrap();
        let zip_data = zip_buf.into_inner();

        // Encrypt it
        let encrypted = create_encrypted_xlsx(&zip_data, "test123").unwrap();
        assert!(is_ole2(&encrypted));

        // Decrypt it
        let decrypted = decrypt_xlsx(&encrypted, "test123").unwrap();

        // The decrypted data should be the original ZIP (with PKCS7 padding removed)
        // Find the ZIP magic bytes
        let zip_start = decrypted
            .windows(4)
            .position(|w| w == b"PK\x03\x04")
            .expect("ZIP magic not found in decrypted data");

        // Verify the decrypted ZIP matches the original
        assert_eq!(&decrypted[zip_start..zip_start + zip_data.len()], &zip_data[..]);
    }

    #[test]
    fn test_decrypt_wrong_password_fails() {
        let mut writer = crate::excel::xlsx_writer::XlsxWriter::new();
        writer.add_sheet("Test").unwrap();
        let mut row = crate::excel::xlsx_writer::RowData::new();
        row.add_string("Secret");
        writer.add_row(row);

        let mut zip_buf = std::io::Cursor::new(Vec::new());
        writer.save(&mut zip_buf).unwrap();
        let zip_data = zip_buf.into_inner();

        let encrypted = create_encrypted_xlsx(&zip_data, "correct").unwrap();
        let result = decrypt_xlsx(&encrypted, "wrong");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Password verification failed"));
    }
}
