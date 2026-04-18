use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// JWT claims — embedded in every token.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // username
    pub iat: usize,  // issued at (unix timestamp)
    pub exp: usize,  // expiration (unix timestamp)
}

/// Database row for the `certificate` table.
#[derive(Debug, FromRow)]
pub struct UserRow {
    #[allow(dead_code)]
    pub username: String,
    pub password: String,
}
