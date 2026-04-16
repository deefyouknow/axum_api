// src/services/auth_service.rs
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use sqlx::PgPool;

use crate::error::AppError;
use crate::models::auth::{Claims, UserRow};

// ── Password ──────────────────────────────────────────────

pub fn hash_password(raw: &str) -> Result<String, AppError> {
    Ok(bcrypt::hash(raw, bcrypt::DEFAULT_COST)?)
}

pub fn verify_password(raw: &str, hash: &str) -> Result<bool, AppError> {
    Ok(bcrypt::verify(raw, hash)?)
}

// ── JWT ───────────────────────────────────────────────────

pub fn generate_jwt(username: &str, secret: &str) -> Result<String, AppError> {
    let now = Utc::now();
    let exp = now + chrono::Duration::hours(24);

    let claims = Claims {
        sub: username.to_owned(),
        iat: now.timestamp() as usize,
        exp: exp.timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok(token)
}

pub fn decode_jwt(token: &str, secret: &str) -> Result<Claims, AppError> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| AppError::Unauthorized("Invalid or expired token".into()))?;

    Ok(data.claims)
}

// ── Database ──────────────────────────────────────────────

pub async fn find_user_by_username(
    pool: &PgPool,
    username: &str,
) -> Result<Option<UserRow>, AppError> {
    let user = sqlx::query_as::<_, UserRow>(
        "SELECT username, password FROM certificate WHERE username = $1 LIMIT 1",
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

pub async fn create_user(
    pool: &PgPool,
    username: &str,
    hashed_password: &str,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO certificate (username, password, role) VALUES ($1, $2, $3)")
        .bind(username)
        .bind(hashed_password)
        .bind("guest")
        .execute(pool)
        .await?;

    Ok(())
}
