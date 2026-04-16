use sqlx::PgPool;

use crate::services::redis_service::Redis;

/// Shared application state — passed to all handlers via Axum's `State` extractor.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub jwt_secret: String,
    pub redis: Option<Redis>,
}
