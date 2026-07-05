// src/handlers/auth_handler.rs
use std::net::SocketAddr;

use axum::{
    Json,
    extract::{ConnectInfo, State},
};

use crate::error::AppError;
use crate::schemas::auth::{AuthResponse, LoginRequest, RegisterRequest};
use crate::services::{auth_service, rate_limit_service};
use crate::state::AppState;

/// POST /auth/register — create a new user account.
pub async fn register(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(body): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // ── Input validation ────────────────────────────────────────────────────
    body.validate()?;

    // ── Rate-limit by IP ─────────────────────────────────────────────────────
    if let Some(ref redis) = state.redis {
        let ip_key = format!("reg_ip:{}", addr.ip());
        rate_limit_service::check_rate_limit(
            redis,
            &ip_key,
            "Too many registration attempts",
        )
        .await?;
    }

    // ── Rate-limit by username ────────────────────────────────────────────────
    if let Some(ref redis) = state.redis {
        let key = format!("reg_attempt:{}", body.username);
        rate_limit_service::check_rate_limit(
            redis,
            &key,
            "Please wait before registering again",
        )
        .await?;
    }

    // ── Hash password ────────────────────────────────────────────────────────
    let hashed = auth_service::hash_password(&body.password).await?;

    // ── Atomic insert — ON CONFLICT prevents race condition (#5) ─────────────
    // Instead of check-then-insert (TOCTOU), use INSERT ON CONFLICT DO NOTHING
    // and check if a row was actually inserted.
    let inserted = auth_service::create_user(&state.db, &body.username, &hashed).await?;

    if !inserted {
        return Err(AppError::BadRequest("Username already taken".into()));
    }

    // ── Set rate-limit keys in Redis ──────────────────────────────────────────
    if let Some(ref redis) = state.redis {
        let ip_key = format!("reg_ip:{}", addr.ip());
        rate_limit_service::set_rate_limit(redis, &ip_key).await;

        let key = format!("reg_attempt:{}", body.username);
        rate_limit_service::set_rate_limit(redis, &key).await;
    }

    // ── Return token so the user is logged in immediately ─────────────────────
    let token = auth_service::generate_jwt(&body.username, &state.jwt_secret)?;
    tracing::info!(username = %body.username, "User registered");

    Ok(Json(AuthResponse { token }))
}

/// POST /auth/login — authenticate and receive a JWT.
pub async fn login(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // ── Rate-limit by IP ─────────────────────────────────────────────────────
    if let Some(ref redis) = state.redis {
        let key = format!("login_ip:{}", addr.ip());
        rate_limit_service::check_rate_limit(
            redis,
            &key,
            "Too many login attempts, please try again later",
        )
        .await?;
    }

    // ── Fetch user from DB ────────────────────────────────────────────────────
    let user = auth_service::find_user_by_username(&state.db, &body.username)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid username or password".into()))?;

    // ── Verify password ───────────────────────────────────────────────────────
    if !auth_service::verify_password(&body.password, &user.password).await? {
        return Err(AppError::Unauthorized(
            "Invalid username or password".into(),
        ));
    }

    // ── Set rate-limit key after successful login ─────────────────────────────
    if let Some(ref redis) = state.redis {
        let key = format!("login_ip:{}", addr.ip());
        rate_limit_service::set_rate_limit(redis, &key).await;
    }

    // ── Generate token ────────────────────────────────────────────────────────
    let token = auth_service::generate_jwt(&body.username, &state.jwt_secret)?;
    tracing::info!(username = %body.username, "User logged in");

    Ok(Json(AuthResponse { token }))
}
