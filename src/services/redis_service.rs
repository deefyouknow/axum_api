// src/services/redis_service.rs
use redis::aio::ConnectionManager;
use redis::Client;

use crate::error::AppError;

/// TTL presets (seconds) — keeps RAM bounded.
pub mod ttl {
    /// Rate-limit keys: 1 minute window
    pub const SHORT: u64 = 60;
    /// Current sensor reading: 60 seconds — ถ้าไม่มีใคร post lux มา 1 นาที ถึงจะล้าง
    pub const SENSOR_CURRENT: u64 = 60;
}

/// Redis key for storing the latest sensor reading (single value, not a list).
pub const SENSOR_CURRENT_KEY: &str = "sensor:current";

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

    // ── Key-value ─────────────────────────────────────────────────────────────

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

    // ── Sensor current reading (single value) ─────────────────────────────────

    /// GET key — read the latest sensor reading from Redis.
    /// Returns None if the key doesn't exist (expired or never set).
    pub async fn get(&self, key: &str) -> Result<Option<String>, AppError> {
        let mut conn = self.conn.clone();
        let val: Option<String> = redis::cmd("GET")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Internal(format!("Redis GET error: {e}")))?;
        Ok(val)
    }
}
