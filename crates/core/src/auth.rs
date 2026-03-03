//! Authentication utilities: password hashing, JWT tokens, refresh tokens.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::models::User;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Token expired")]
    TokenExpired,
    #[error("Invalid token: {0}")]
    InvalidToken(String),
    #[error("Registration disabled")]
    RegistrationDisabled,
    #[error("Email already registered")]
    EmailAlreadyExists,
    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String, // user_id
    pub email: String,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub user: User,
}

// ── Password hashing ─────────────────────────────────────────────

pub fn hash_password(password: &str) -> Result<String, AuthError> {
    use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
    let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| AuthError::Internal(format!("Password hashing failed: {e}")))
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::{Argon2, PasswordVerifier, PasswordHash};
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

// ── JWT tokens ───────────────────────────────────────────────────

pub fn create_access_token(
    user_id: &str,
    email: &str,
    secret: &str,
    duration_secs: u64,
) -> Result<String, AuthError> {
    let now = chrono::Utc::now();
    let claims = JwtClaims {
        sub: user_id.to_string(),
        email: email.to_string(),
        iat: now.timestamp() as usize,
        exp: (now + chrono::Duration::seconds(duration_secs as i64)).timestamp() as usize,
    };
    let key = jsonwebtoken::EncodingKey::from_secret(secret.as_bytes());
    jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &key)
        .map_err(|e| AuthError::Internal(format!("JWT encoding failed: {e}")))
}

pub fn validate_access_token(token: &str, secret: &str) -> Result<JwtClaims, AuthError> {
    let key = jsonwebtoken::DecodingKey::from_secret(secret.as_bytes());
    let validation = jsonwebtoken::Validation::default();
    jsonwebtoken::decode::<JwtClaims>(token, &key, &validation)
        .map(|data| data.claims)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
            _ => AuthError::InvalidToken(e.to_string()),
        })
}

// ── Refresh tokens ───────────────────────────────────────────────

pub fn generate_refresh_token() -> String {
    use rand::Fill as _;
    let mut bytes = [0u8; 32];
    bytes.fill(&mut rand::rng());
    hex::encode(bytes)
}

pub fn hash_refresh_token(token: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hash_verify() {
        let hash = hash_password("test_password").unwrap();
        assert!(verify_password("test_password", &hash));
        assert!(!verify_password("wrong_password", &hash));
    }

    #[test]
    fn test_jwt_roundtrip() {
        let secret = "test_secret_key_for_jwt";
        let token = create_access_token("user-123", "test@example.com", secret, 3600).unwrap();
        let claims = validate_access_token(&token, secret).unwrap();
        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.email, "test@example.com");
    }

    #[test]
    fn test_jwt_wrong_secret() {
        let token = create_access_token("user-123", "test@example.com", "secret1", 3600).unwrap();
        assert!(validate_access_token(&token, "secret2").is_err());
    }

    #[test]
    fn test_refresh_token_generation() {
        let token = generate_refresh_token();
        assert_eq!(token.len(), 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn test_refresh_token_hash() {
        let token = generate_refresh_token();
        let hash = hash_refresh_token(&token);
        assert_eq!(hash.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
        // Same token produces same hash
        assert_eq!(hash, hash_refresh_token(&token));
    }
}
