use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// JWT claims — embedded in every token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // username
    pub iat: i64,    // issued at (unix timestamp)
    pub exp: i64,    // expiration (unix timestamp)
}

/// Database row for the `certificate` table.
#[derive(Debug, FromRow)]
pub struct UserRow {
    #[allow(dead_code)]
    pub username: String,
    pub password: String,
}
