// src/services/redis_service.rs
use redis::aio::ConnectionManager;
use redis::Client;

use crate::error::AppError;

/// TTL presets (seconds) — keeps RAM bounded.
pub mod ttl {
    pub const SHORT: u64 = 60; // 1 min — rate-limit keys
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

    /// SET key value EX ttl — mandatory TTL (seconds) prevents orphan keys eating RAM.
    pub async fn set(&self, key: &str, value: &str, ttl_secs: u64) -> Result<(), AppError> {
        let mut conn = self.conn.clone();
        let _: () = redis::cmd("SET")
            .arg(key)
            .arg(value)
            .arg("EX")
            .arg(ttl_secs)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Internal(format!("Redis SET error: {e}")))?;
        Ok(())
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
