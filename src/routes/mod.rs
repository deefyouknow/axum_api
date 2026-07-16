use axum::{Router, routing::{post, get, patch}};

use crate::handlers::auth_handler;
use crate::handlers::command_handler;
use crate::handlers::sensor_handler;
use crate::state::AppState;

async fn health() -> &'static str {
    "ok"
}

/// Public routes — no authentication required.
pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(auth_handler::register))
        .route("/auth/login", post(auth_handler::login))
        .route("/sensors/reading", post(sensor_handler::post_sensor_reading))
        .route("/health", get(health))
}

/// Protected routes — require valid JWT in Authorization header.
pub fn protected_routes() -> Router<AppState> {
    Router::new()
        .route("/sensors/history", get(sensor_handler::get_sensor_history))
        .route("/sensors/latest", get(sensor_handler::get_sensor_latest))
        .route("/sensors/available-dates", get(sensor_handler::get_available_dates))
        .route("/commands", post(command_handler::create_command))
        .route("/commands/pending", get(command_handler::get_pending_commands))
        .route("/commands/{id}", get(command_handler::get_command_by_id))
        .route("/commands/{id}/response", patch(command_handler::update_command_response))
        .route("/commands/history", get(command_handler::get_command_history))
}
