// src/services/rate_limit_service.rs
use crate::error::AppError;
use crate::services::redis_service::{Redis, ttl};

/// Check if a rate-limit key already exists.
/// Returns `Err(AppError::BadRequest)` if the key exists (rate limit hit).
pub async fn check_rate_limit(redis: &Redis, key: &str, error_msg: &str) -> Result<(), AppError> {
    if redis.exists(key).await? {
        return Err(AppError::BadRequest(error_msg.into()));
    }
    Ok(())
}

/// Set a rate-limit key with the default SHORT TTL.
/// Logs a warning on Redis failure but never crashes.
pub async fn set_rate_limit(redis: &Redis, key: &str) {
    if let Err(e) = redis.set(key, "1", ttl::SHORT).await {
        tracing::warn!("Redis SET {key} failed: {e}");
    }
}
