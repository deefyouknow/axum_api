use axum::{Router, routing::post};

use crate::handlers::auth_handler;
use crate::state::AppState;

/// All `/auth` routes.
pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(auth_handler::register))
        .route("/auth/login", post(auth_handler::login))
}
