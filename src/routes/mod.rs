use axum::{Router, routing::{post, get}};

use crate::handlers::auth_handler;
use crate::state::AppState;

async fn health() -> &'static str {
    "ok"
}
/// All `/auth` routes.
pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(auth_handler::register))
        .route("/auth/login", post(auth_handler::login))
        .route("/health", get(health))
}
