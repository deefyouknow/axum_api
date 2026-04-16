use redis::aio::ConnectionManager;
use redis::Client;

use crate::error::AppError;

/// TTL presets (seconds) — keeps RAM bounded.
pub mod ttl {
    pub const SHORT: u64 = 60;        // 1 min  — rate-limit keys, OTP
    pub const MEDIUM: u64 = 300;      // 5 min  — login cache
    pub const LONG: u64 = 3600;       // 1 hour — session data
    pub const MAX: u64 = 86400;       // 24 hr  — daily counters
}

/// Thin Redis wrapper — all writes require a TTL.
#[derive(Clone)]
pub struct Redis {
    conn: ConnectionManager,
}

impl Redis {
    /// Connect to Redis and return a ready-to-use handle.
    pub async fn connect(url: &str) -> Result<Self, AppError> {
        let client = Client::open(url)
            .map_err(|e| AppError::Internal(format!("Redis client error: {e}")))?;

        let conn = ConnectionManager::new(client)
            .await
            .map_err(|e| AppError::Internal(format!("Redis connection error: {e}")))?;

        Ok(Self { conn })
    }

    /// SET with mandatory TTL (seconds) — prevents orphan keys eating RAM.
    pub async fn set(&self, key: &str, value: &str, ttl_secs: u64) -> Result<(), AppError> {
        let mut conn = self.conn.clone();
        let _: () = redis::cmd("SETEX")
            .arg(key)
            .arg(ttl_secs)
            .arg(value)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Internal(format!("Redis SET error: {e}")))?;
        Ok(())
    }

    /// GET — returns None on cache miss.
    pub async fn get(&self, key: &str) -> Result<Option<String>, AppError> {
        let mut conn = self.conn.clone();
        let result: Option<String> = redis::cmd("GET")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Internal(format!("Redis GET error: {e}")))?;
        Ok(result)
    }

    /// DEL — returns true if the key existed.
    pub async fn del(&self, key: &str) -> Result<bool, AppError> {
        let mut conn = self.conn.clone();
        let result: i32 = redis::cmd("DEL")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Internal(format!("Redis DEL error: {e}")))?;
        Ok(result > 0)
    }

    /// EXISTS — check if a key is present.
    pub async fn exists(&self, key: &str) -> Result<bool, AppError> {
        let mut conn = self.conn.clone();
        let result: i32 = redis::cmd("EXISTS")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Internal(format!("Redis EXISTS error: {e}")))?;
        Ok(result > 0)
    }
}
