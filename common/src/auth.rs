use anyhow::Context;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub email: String,
    pub role: String,
    pub iat: i64,
    pub exp: i64,
}

pub struct JwtConfig {
    encoding: EncodingKey,
    decoding: DecodingKey,
    pub access_ttl_secs: i64,
}

impl JwtConfig {
    pub fn from_secret(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
            access_ttl_secs: 15 * 60,
        }
    }

    pub fn encode(&self, claims: &Claims) -> anyhow::Result<String> {
        encode(&Header::default(), claims, &self.encoding).context("jwt encode")
    }

    pub fn decode(&self, token: &str) -> anyhow::Result<Claims> {
        let data = decode::<Claims>(token, &self.decoding, &Validation::default())
            .context("jwt decode")?;
        Ok(data.claims)
    }

    pub fn make_claims(&self, user_id: Uuid, email: &str, role: &str) -> Claims {
        let now = Utc::now().timestamp();
        Claims {
            sub: user_id,
            email: email.to_owned(),
            role: role.to_owned(),
            iat: now,
            exp: now + self.access_ttl_secs,
        }
    }
}

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| anyhow::anyhow!("hash_password: {e}"))
}

pub fn verify_password(password: &str, hash: &str) -> anyhow::Result<bool> {
    let parsed = PasswordHash::new(hash).map_err(|e| anyhow::anyhow!("bad hash: {e}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

pub fn generate_refresh_token() -> String {
    let bytes: [u8; 32] = rand::thread_rng().gen();
    hex::encode(bytes)
}

pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}
