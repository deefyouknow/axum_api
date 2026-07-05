// src/middleware/auth.rs
use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};

use crate::error::AppError;
use crate::services::auth_service;
use crate::state::AppState;

/// JWT authentication middleware.
///
/// Extracts the Bearer token from the Authorization header, validates it,
/// and inserts the decoded `Claims` into the request extensions for
/// downstream handlers to use.
///
/// Returns 401 Unauthorized if the token is missing, malformed, or expired.
pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let header_value = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".into()))?;

    let token = header_value
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("Invalid Authorization format".into()))?;

    let claims = auth_service::decode_jwt(token, &state.jwt_secret)?;
    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}
