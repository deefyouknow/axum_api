use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// POST /auth/login request body.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// POST /auth/register request body.
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

impl RegisterRequest {
    /// Validate registration input.
    /// Returns `Ok(())` if valid, or `Err(AppError)` with a descriptive message.
    pub fn validate(&self) -> Result<(), AppError> {
        let username = self.username.trim();
        if username.is_empty() {
            return Err(AppError::BadRequest(
                "Username must not be empty".into(),
            ));
        }
        if username.len() > 50 {
            return Err(AppError::BadRequest(
                "Username must not exceed 50 characters".into(),
            ));
        }
        if self.password.len() < 8 {
            return Err(AppError::BadRequest(
                "Password must be at least 8 characters".into(),
            ));
        }
        if self.password.len() > 128 {
            return Err(AppError::BadRequest(
                "Password must not exceed 128 characters".into(),
            ));
        }
        Ok(())
    }
}

/// Auth response — returned on successful login or register.
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
}
