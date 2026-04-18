// src/handlers/auth_handler.rs
use std::net::SocketAddr;

use axum::{
    Json,
    extract::{ConnectInfo, State},
};

use crate::error::AppError;
use crate::schemas::auth::{AuthResponse, LoginRequest, RegisterRequest};
use crate::services::{auth_service, redis_service::ttl};
use crate::state::AppState;

/// POST /auth/register — create a new user account.
pub async fn register(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(body): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // ── Rate-limit by IP ─────────────────────────────────────────────────────
    // Block repeated registrations from the same IP within the TTL window
    if let Some(ref redis) = state.redis {
        let ip_key = format!("reg_ip:{}", addr.ip());
        if redis.exists(&ip_key).await? {
            return Err(AppError::BadRequest(
                "Too many registration attempts".into(),
            ));
        }
    }

    // ── Rate-limit by username ────────────────────────────────────────────────
    // Block re-registration of the same username within the TTL window
    if let Some(ref redis) = state.redis {
        let key = format!("reg_attempt:{}", body.username);
        if redis.exists(&key).await? {
            return Err(AppError::BadRequest(
                "Please wait before registering again".into(),
            ));
        }
    }

    // ── Uniqueness check ──────────────────────────────────────────────────────
    // Ensure no existing account shares the requested username
    if auth_service::find_user_by_username(&state.db, &body.username)
        .await?
        .is_some()
    {
        return Err(AppError::BadRequest("Username already taken".into()));
    }

    // ── Hash password and persist ─────────────────────────────────────────────
    let hashed = auth_service::hash_password(&body.password).await?;
    auth_service::create_user(&state.db, &body.username, &hashed).await?;

    // ── Set rate-limit keys in Redis ──────────────────────────────────────────
    // Both IP and username keys are written after a successful registration
    if let Some(ref redis) = state.redis {
        let ip_key = format!("reg_ip:{}", addr.ip());
        let _ = redis.set(&ip_key, "1", ttl::SHORT).await;

        let key = format!("reg_attempt:{}", body.username);
        let _ = redis.set(&key, "1", ttl::SHORT).await;
    }

    // ── Return token so the user is logged in immediately ─────────────────────
    let token = auth_service::generate_jwt(&body.username, &state.jwt_secret)?;
    tracing::info!(username = %body.username, "User registered");

    Ok(Json(AuthResponse { token }))
}

/// POST /auth/login — authenticate and receive a JWT.
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // ── Fetch user from DB ────────────────────────────────────────────────────
    // Always query the database directly; bcrypt hashes must never be cached
    let user = auth_service::find_user_by_username(&state.db, &body.username)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid username or password".into()))?;

    // ── Verify password ───────────────────────────────────────────────────────
    if !auth_service::verify_password(&body.password, &user.password).await? {
        return Err(AppError::Unauthorized(
            "Invalid username or password".into(),
        ));
    }

    // ── Generate token ────────────────────────────────────────────────────────
    let token = auth_service::generate_jwt(&body.username, &state.jwt_secret)?;
    tracing::info!(username = %body.username, "User logged in");

    Ok(Json(AuthResponse { token }))
}
