use serde::{Deserialize, Serialize};

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

/// Auth response — returned on successful login or register.
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
}
