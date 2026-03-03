use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::Aead,
};
use rand::Rng;

use crate::models::AuthConfig;

pub fn encrypt(plaintext: &str, key: &[u8; 32]) -> Result<String, String> {
    let cipher = Aes256Gcm::new(key.into());
    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(format!("{}:{}", hex::encode(nonce_bytes), hex::encode(ciphertext)))
}

pub fn decrypt(encrypted: &str, key: &[u8; 32]) -> Result<String, String> {
    let parts: Vec<&str> = encrypted.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err("invalid encrypted format".into());
    }
    let nonce_bytes = hex::decode(parts[0]).map_err(|e| e.to_string())?;
    let ciphertext = hex::decode(parts[1]).map_err(|e| e.to_string())?;
    let cipher = Aes256Gcm::new(key.into());
    let nonce = Nonce::from_slice(&nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| e.to_string())?;
    String::from_utf8(plaintext).map_err(|e| e.to_string())
}

fn encrypt_field(value: &str, key: &[u8; 32]) -> Result<String, String> {
    if value.is_empty() {
        return Ok(String::new());
    }
    Ok(format!("enc:{}", encrypt(value, key)?))
}

fn decrypt_field(value: &str, key: &[u8; 32]) -> String {
    if let Some(inner) = value.strip_prefix("enc:") {
        decrypt(inner, key).unwrap_or_else(|_| value.to_string())
    } else {
        value.to_string()
    }
}

pub fn encrypt_auth_secrets(auth: &AuthConfig, key: &[u8; 32]) -> Result<AuthConfig, String> {
    Ok(match auth {
        AuthConfig::None => AuthConfig::None,
        AuthConfig::Basic { username, password } => AuthConfig::Basic {
            username: username.clone(),
            password: encrypt_field(password, key)?,
        },
        AuthConfig::Bearer { token } => AuthConfig::Bearer {
            token: encrypt_field(token, key)?,
        },
        AuthConfig::ApiKey {
            key: k,
            value,
            add_to,
        } => AuthConfig::ApiKey {
            key: k.clone(),
            value: encrypt_field(value, key)?,
            add_to: add_to.clone(),
        },
        AuthConfig::OAuth2ClientCredentials {
            token_url,
            client_id,
            client_secret,
            scope,
        } => AuthConfig::OAuth2ClientCredentials {
            token_url: token_url.clone(),
            client_id: client_id.clone(),
            client_secret: encrypt_field(client_secret, key)?,
            scope: scope.clone(),
        },
    })
}

pub fn decrypt_auth_secrets(auth: &AuthConfig, key: &[u8; 32]) -> AuthConfig {
    match auth {
        AuthConfig::None => AuthConfig::None,
        AuthConfig::Basic { username, password } => AuthConfig::Basic {
            username: username.clone(),
            password: decrypt_field(password, key),
        },
        AuthConfig::Bearer { token } => AuthConfig::Bearer {
            token: decrypt_field(token, key),
        },
        AuthConfig::ApiKey {
            key: k,
            value,
            add_to,
        } => AuthConfig::ApiKey {
            key: k.clone(),
            value: decrypt_field(value, key),
            add_to: add_to.clone(),
        },
        AuthConfig::OAuth2ClientCredentials {
            token_url,
            client_id,
            client_secret,
            scope,
        } => AuthConfig::OAuth2ClientCredentials {
            token_url: token_url.clone(),
            client_id: client_id.clone(),
            client_secret: decrypt_field(client_secret, key),
            scope: scope.clone(),
        },
    }
}
