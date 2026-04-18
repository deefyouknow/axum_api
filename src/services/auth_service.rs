// src/services/auth_service.rs
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use sqlx::PgPool;
use tokio::task;

use crate::error::AppError;
use crate::models::auth::{Claims, UserRow};

// ── Password ──────────────────────────────────────────────

// ย้าย bcrypt ไปรันบน blocking thread pool — ป้องกันไม่ให้บล็อก tokio worker thread
// bcrypt เป็น CPU-heavy sync operation ใช้เวลา ~250-400ms ต่อครั้ง
pub async fn hash_password(raw: &str) -> Result<String, AppError> {
    let raw = raw.to_owned();
    task::spawn_blocking(move || {
        bcrypt::hash(&raw, bcrypt::DEFAULT_COST)
            .map_err(AppError::from)
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking error: {e}")))?
}

pub async fn verify_password(raw: &str, hash: &str) -> Result<bool, AppError> {
    let raw = raw.to_owned();
    let hash = hash.to_owned();
    task::spawn_blocking(move || {
        bcrypt::verify(&raw, &hash)
            .map_err(AppError::from)
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking error: {e}")))?
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

#[allow(dead_code)]
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
