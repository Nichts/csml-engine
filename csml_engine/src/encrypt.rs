/**
 * Encryption and decryption utils for securely storing sensitive data.
 * It is not mandatory to setup encryption in CSML engine, however it is greatly recommended
 * if the chatbot is handling sensitive data of any sort.
 *
 * To automatically setup encryption/decryption of data, you must set an ENCRYPTION_SECRET
 * environment variable with a complex enough string.
 *
 * Encrypt: Data is JSON-stringified before encryption, and is returned as an encrypted string.
 * Decrypt: Data is decrypted from an encrypted string and is returned as a JSON Value.
 *
 * The encryption algorithm used is AES-256-GCM.
 */
use crate::EngineError;

#[cfg(all(feature = "openssl", not(feature = "rustls")))]
use openssl::{
    pkcs5::pbkdf2_hmac,
    rand::rand_bytes,
    symm::{decrypt_aead, encrypt_aead, Cipher},
};

use std::env;
#[cfg(feature = "rustls")]
use std::num::NonZeroU32;

#[cfg(feature = "rustls")]
use aes_gcm::{
    aead::{consts::U16, AeadCore, KeyInit, Payload},
    aes::{cipher::Unsigned, Aes256},
    AeadInPlace, AesGcm,
};

use base64::Engine;
#[cfg(feature = "rustls")]
use rand::{rngs::OsRng, RngCore};
#[cfg(feature = "rustls")]
use ring::pbkdf2;

#[cfg(feature = "rustls")]
static PBKDF2_ALG: pbkdf2::Algorithm = pbkdf2::PBKDF2_HMAC_SHA512;
// static PBKDF2_ITERATIONS: NonZeroU32 = NonZeroU32::new(10000).unwrap();

// AES-GCM with a 256-bit key and 128-bit nonce.
#[cfg(feature = "rustls")]
type Aes256Gcm16 = AesGcm<Aes256, U16>;

fn get_encryption_secret() -> String {
    match env::var("ENCRYPTION_SECRET") {
        Ok(var) => var,
        _ => panic!("No ENCRYPTION_SECRET value in env"),
    }
}

#[cfg(all(feature = "openssl", not(feature = "rustls")))]
fn get_key(salt: &[u8], key: &mut [u8]) -> Result<(), EngineError> {
    let pass = get_encryption_secret();

    pbkdf2_hmac(
        pass.as_bytes(),
        salt,
        10000,
        openssl::hash::MessageDigest::sha512(),
        key,
    )?;

    Ok(())
}

#[cfg(feature = "rustls")]
fn get_key(salt: &[u8], key: &mut [u8]) -> Result<(), EngineError> {
    let pass = get_encryption_secret();

    pbkdf2::derive(
        PBKDF2_ALG,
        NonZeroU32::new(10000).unwrap(),
        salt,
        pass.as_bytes(),
        key,
    );

    Ok(())
}

/**
 * Decode base64 or hex-encoded strings.
 * The legacy engine used hex encoding which must still be decoded properly
 * so in case b64 does not work, try hex as well before returning an error.
 * This will not impact performance of newly-encrypted data, while
 * retaining full retrocompatibility with older data at a small cost.
 */
fn decode(text: &str) -> Result<Vec<u8>, EngineError> {
    match hex::decode(text) {
        Ok(val) => Ok(val),
        Err(_) => match base64::engine::general_purpose::STANDARD.decode(text) {
            Ok(val) => Ok(val),
            Err(err) => Err(EngineError::Base64(err)),
        },
    }
}

#[cfg(all(feature = "openssl", not(feature = "rustls")))]
fn encrypt(text: &[u8]) -> Result<String, EngineError> {
    let cipher = Cipher::aes_256_gcm();

    let mut tag = vec![0; 16];
    let mut iv = vec![0; 16];
    rand_bytes(&mut iv)?;
    let mut salt = vec![0; 64];
    rand_bytes(&mut salt)?;
    let mut key = [0; 32];
    get_key(&salt, &mut key)?;

    let encrypted = encrypt_aead(cipher, &key, Some(&iv), &[], text, &mut tag)?;

    Ok(base64::engine::general_purpose::STANDARD.encode([salt, iv, tag, encrypted].concat()))
}

#[cfg(feature = "rustls")]
fn encrypt(text: &[u8]) -> Result<String, EngineError> {
    let mut key = [0; 32];
    let mut salt = vec![0; 64];
    OsRng.fill_bytes(&mut salt);
    get_key(&salt, &mut key)?;

    let cipher = Aes256Gcm16::new(&key.into());
    let nonce = Aes256Gcm16::generate_nonce(&mut OsRng);

    // Unrolled version of let encrypted = cipher.encrypt(&nonce, text)?;
    let payload: Payload = text.into();
    let mut encrypted =
        Vec::with_capacity(payload.msg.len() + <Aes256Gcm16 as AeadCore>::TagSize::to_usize());
    encrypted.extend_from_slice(payload.msg);
    let tag = cipher.encrypt_in_place_detached(&nonce, &[], encrypted.as_mut())?;

    Ok(base64::engine::general_purpose::STANDARD
        .encode([salt, nonce.to_vec(), tag.to_vec(), encrypted].concat()))
}

pub fn encrypt_data(value: &serde_json::Value) -> Result<String, EngineError> {
    match env::var("ENCRYPTION_SECRET") {
        Ok(..) => encrypt(value.to_string().as_bytes()),
        _ => Ok(value.to_string()),
    }
}

#[cfg(all(feature = "openssl", not(feature = "rustls")))]
fn decrypt(text: String) -> Result<String, EngineError> {
    let ciphertext = decode(&text)?;
    let cipher = Cipher::aes_256_gcm();

    let iv_length = 16;
    let salt_length = 64;
    let tag_length = 16;
    let tag_position = salt_length + iv_length;
    let encrypted_position = tag_position + tag_length;

    let salt: &[u8] = &ciphertext[0..salt_length];
    let iv: &[u8] = &ciphertext[salt_length..tag_position];
    let tag: &[u8] = &ciphertext[tag_position..encrypted_position];
    let encrypted: &[u8] = &ciphertext[encrypted_position..];

    let mut key = [0; 32];
    get_key(salt, &mut key)?;

    let value = decrypt_aead(cipher, &key, Some(iv), &[], encrypted, tag)?;

    Ok(String::from_utf8_lossy(&value).to_string())
}

#[cfg(feature = "rustls")]
fn decrypt(text: String) -> Result<String, EngineError> {
    let ciphertext = decode(&text)?;

    let nonce_length = 16;
    let salt_length = 64;
    let tag_length = 16;
    let tag_position = salt_length + nonce_length;
    let encrypted_position = tag_position + tag_length;

    let salt: &[u8] = &ciphertext[0..salt_length];
    let nonce: &[u8] = &ciphertext[salt_length..tag_position];
    let tag: &[u8] = &ciphertext[tag_position..encrypted_position];
    let mut buffer = ciphertext[encrypted_position..].to_vec();

    let mut key = [0; 32];
    get_key(salt, &mut key)?;

    let cipher = Aes256Gcm16::new(&key.into());
    // Unrolled version of let encrypted = cipher.decrypt(...)?;
    cipher.decrypt_in_place_detached(nonce.into(), &[], buffer.as_mut_slice(), tag.into())?;

    Ok(String::from_utf8_lossy(&buffer).to_string())
}

pub fn decrypt_data(value: String) -> Result<serde_json::Value, EngineError> {
    match env::var("ENCRYPTION_SECRET") {
        Ok(..) => {
            let value: serde_json::Value = serde_json::from_str(&decrypt(value)?)?;
            Ok(value)
        }
        _ => {
            let value: serde_json::Value = serde_json::from_str(&value)?;
            Ok(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_key() -> Result<(), EngineError> {
        env::set_var("ENCRYPTION_SECRET", "test");
        let salt = b"test123";
        let key = b"key123".to_vec();
        let mut encrypted = key.clone();

        get_key(salt, encrypted.as_mut_slice())?;

        assert_eq!(encrypted, [35, 47, 70, 158, 170, 34]);
        Ok(())
    }

    #[test]
    fn test_encrypt_decrypt() {
        env::set_var("ENCRYPTION_SECRET", "test");
        let text = "text".to_owned();

        let encrypted = encrypt(text.as_bytes()).unwrap();
        let decrypted = decrypt(encrypted).unwrap();

        assert_eq!(text, decrypted);
    }
}
