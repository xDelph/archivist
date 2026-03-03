use std::env;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::{Context, Result, anyhow, bail};
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::{Digest, Sha256};

const EMAIL_LOOKUP_KEY_ENV: &str = "EMAIL_LOOKUP_KEY";
const EMAIL_ENCRYPTION_KEY_ENV: &str = "EMAIL_ENCRYPTION_KEY";
const PASSWORD_PEPPER_ENV: &str = "PASSWORD_PEPPER";
const ENCRYPTION_FORMAT_VERSION: &str = "v1";

type HmacSha256 = Hmac<Sha256>;

pub fn normalize_email(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub fn email_lookup_hash(normalized_email: &str) -> Result<String> {
    let key = read_key(EMAIL_LOOKUP_KEY_ENV)?;
    let mut mac = <HmacSha256 as Mac>::new_from_slice(&key)
        .map_err(|_| anyhow!("invalid {} value", EMAIL_LOOKUP_KEY_ENV))?;
    mac.update(normalized_email.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

pub fn encrypt_email(normalized_email: &str) -> Result<String> {
    let key = read_32_byte_key(EMAIL_ENCRYPTION_KEY_ENV)?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| anyhow!("invalid {} value", EMAIL_ENCRYPTION_KEY_ENV))?;

    let mut nonce = [0_u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let nonce_ref = Nonce::from_slice(&nonce);
    let ciphertext = cipher
        .encrypt(nonce_ref, normalized_email.as_bytes())
        .map_err(|_| anyhow!("email encryption failure"))?;
    Ok(format!(
        "{}:{}:{}",
        ENCRYPTION_FORMAT_VERSION,
        URL_SAFE_NO_PAD.encode(nonce),
        URL_SAFE_NO_PAD.encode(ciphertext)
    ))
}

pub fn decrypt_email(ciphertext: &str) -> Result<String> {
    let key = read_32_byte_key(EMAIL_ENCRYPTION_KEY_ENV)?;
    let mut parts = ciphertext.split(':');
    let version = parts.next().unwrap_or_default();
    if version != ENCRYPTION_FORMAT_VERSION {
        bail!("unsupported email encryption format");
    }
    let nonce_b64 = parts.next().unwrap_or_default();
    let ct_b64 = parts.next().unwrap_or_default();
    if parts.next().is_some() {
        bail!("malformed encrypted email payload");
    }

    let nonce = URL_SAFE_NO_PAD
        .decode(nonce_b64)
        .context("invalid encrypted email nonce")?;
    let ct = URL_SAFE_NO_PAD
        .decode(ct_b64)
        .context("invalid encrypted email payload")?;
    if nonce.len() != 12 {
        bail!("invalid encrypted email nonce length");
    }

    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| anyhow!("invalid {} value", EMAIL_ENCRYPTION_KEY_ENV))?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce), ct.as_ref())
        .map_err(|_| anyhow!("email decryption failure"))?;
    String::from_utf8(plaintext).context("invalid encrypted email plaintext")
}

pub fn hash_password(password: &str) -> Result<String> {
    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);
    let secret = peppered_password(password);
    Ok(argon2
        .hash_password(secret.as_bytes(), &salt)
        .map_err(|e| anyhow!("password hashing failed: {e}"))?
        .to_string())
}

pub fn verify_password(password: &str, expected_hash: &str) -> bool {
    let parsed = match PasswordHash::new(expected_hash) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let secret = peppered_password(password);
    Argon2::default()
        .verify_password(secret.as_bytes(), &parsed)
        .is_ok()
}

pub fn generate_token() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn sha256_hex(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    hex::encode(digest)
}

fn peppered_password(password: &str) -> String {
    let pepper = env::var(PASSWORD_PEPPER_ENV).unwrap_or_default();
    if pepper.is_empty() {
        password.to_owned()
    } else {
        format!("{password}:{pepper}")
    }
}

fn read_key(env_key: &str) -> Result<Vec<u8>> {
    let raw = env::var(env_key).with_context(|| format!("missing {env_key}"))?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("missing {env_key}");
    }

    if let Ok(decoded) = URL_SAFE_NO_PAD.decode(trimmed)
        && !decoded.is_empty()
    {
        return Ok(decoded);
    }
    if let Ok(decoded) = STANDARD.decode(trimmed)
        && !decoded.is_empty()
    {
        return Ok(decoded);
    }
    Ok(trimmed.as_bytes().to_vec())
}

fn read_32_byte_key(env_key: &str) -> Result<[u8; 32]> {
    let key = read_key(env_key)?;
    if key.len() != 32 {
        bail!("{env_key} must resolve to exactly 32 bytes");
    }
    let mut out = [0_u8; 32];
    out.copy_from_slice(&key);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_email_trims_and_lowercases() {
        assert_eq!(normalize_email("  Foo.Bar@Acme.Com "), "foo.bar@acme.com");
    }

    #[test]
    fn password_hash_and_verify_roundtrip() {
        let hash = hash_password("supersafe-password").expect("password hash should succeed");
        assert!(verify_password("supersafe-password", &hash));
        assert!(!verify_password("wrong-password", &hash));
    }

    #[test]
    fn token_generation_returns_value() {
        let token = generate_token();
        assert!(token.len() >= 32);
    }
}
