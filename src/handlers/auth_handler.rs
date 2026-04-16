// src/handlers/auth_handler.rs
use axum::{Json, extract::State};

use crate::error::AppError;
use crate::schemas::auth::{AuthResponse, LoginRequest, RegisterRequest};
use crate::services::{auth_service, redis_service::ttl};
use crate::state::AppState;

/// POST /auth/register — create a new user account.
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // Rate-limit: block re-registration within 60s
    if let Some(ref redis) = state.redis {
        let key = format!("reg_attempt:{}", body.username);
        if redis.exists(&key).await? {
            return Err(AppError::BadRequest("Please wait before registering again".into()));
        }
    }

    // Check if user already exists
    if auth_service::find_user_by_username(&state.db, &body.username)
        .await?
        .is_some()
    {
        return Err(AppError::BadRequest("Username already taken".into()));
    }

    // Hash password and persist
    let hashed = auth_service::hash_password(&body.password)?;
    auth_service::create_user(&state.db, &body.username, &hashed).await?;

    // Set rate-limit key in Redis
    if let Some(ref redis) = state.redis {
        let key = format!("reg_attempt:{}", body.username);
        let _ = redis.set(&key, "1", ttl::SHORT).await;
    }

    // Return a token so the user is logged in immediately
    let token = auth_service::generate_jwt(&body.username, &state.jwt_secret)?;
    tracing::info!(username = %body.username, "User registered");

    Ok(Json(AuthResponse { token }))
}

/// POST /auth/login — authenticate and receive a JWT.
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // Try Redis cache first
    let cached_password = if let Some(ref redis) = state.redis {
        let key = format!("user_pw:{}", body.username);
        redis.get(&key).await?
    } else {
        None
    };

    let db_password = match cached_password {
        Some(pw) => {
            tracing::debug!(username = %body.username, "Cache hit");
            pw
        }
        None => {
            // Cache miss — query DB
            let user = auth_service::find_user_by_username(&state.db, &body.username)
                .await?
                .ok_or_else(|| AppError::Unauthorized("Invalid username or password".into()))?;

            // Cache the hashed password for next login (TTL: 5 min)
            if let Some(ref redis) = state.redis {
                let key = format!("user_pw:{}", body.username);
                let _ = redis.set(&key, &user.password, ttl::MEDIUM).await;
            }

            user.password
        }
    };

    // Verify password
    if !auth_service::verify_password(&body.password, &db_password)? {
        return Err(AppError::Unauthorized("Invalid username or password".into()));
    }

    // Generate token
    let token = auth_service::generate_jwt(&body.username, &state.jwt_secret)?;
    tracing::info!(username = %body.username, "User logged in");

    Ok(Json(AuthResponse { token }))
}
