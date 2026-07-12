// src/services/redis_service.rs
use redis::aio::ConnectionManager;
use redis::{Client, Pipeline};

use crate::error::AppError;

/// TTL presets (seconds) — keeps RAM bounded.
pub mod ttl {
    /// Rate-limit keys: 1 minute window
    pub const SHORT: u64 = 60;
    /// Sensor write-buffer: 5 minutes — enough to survive a brief DB outage
    /// and drain automatically if the flusher is not running.
    pub const SENSOR_BUFFER: u64 = 300;
}

/// Maximum number of rows pulled from the buffer in one flush cycle.
/// 100 rows × ~200 bytes ≈ 20 KB — safe for a single INSERT statement.
pub const SENSOR_BUFFER_BATCH: isize = 100;

/// Redis list key used as the sensor write-buffer.
pub const SENSOR_BUFFER_KEY: &str = "sensor:buffer";

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

    // ── List (sensor write-buffer) ────────────────────────────────────────────

    /// LPUSH key value — push a JSON string onto the head of a Redis list.
    /// Also resets the TTL so the key doesn't expire while data is accumulating.
    pub async fn lpush(&self, key: &str, value: &str, ttl_secs: u64) -> Result<(), AppError> {
        let mut conn = self.conn.clone();
        // Use a pipeline: LPUSH + EXPIRE in one round-trip
        let _: (i64, bool) = Pipeline::new()
            .lpush(key, value)
            .expire(key, ttl_secs as i64)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Internal(format!("Redis LPUSH error: {e}")))?;
        Ok(())
    }

    /// Atomically pull up to `count` items from the *tail* of the list (oldest first,
    /// because we LPUSH) and trim them away — returns the drained items.
    ///
    /// Uses a Lua script so LRANGE + LTRIM are atomic: no item is lost even if the
    /// server crashes between the two commands.
    pub async fn lrange_and_trim(
        &self,
        key: &str,
        count: isize,
    ) -> Result<Vec<String>, AppError> {
        let mut conn = self.conn.clone();

        // Lua: atomically read the oldest `count` items then remove them.
        // Items are stored newest-first (LPUSH), so "oldest" = tail of list.
        // LRANGE -count -1  → last `count` elements (oldest)
        // LTRIM  0  -(count+1) → keep everything except those last `count` elements
        let script = redis::Script::new(
            r#"
            local key   = KEYS[1]
            local n     = tonumber(ARGV[1])
            local items = redis.call('LRANGE', key, -n, -1)
            if #items > 0 then
                redis.call('LTRIM', key, 0, -(#items + 1))
            end
            return items
            "#,
        );

        let items: Vec<String> = script
            .key(key)
            .arg(count)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| AppError::Internal(format!("Redis lrange_and_trim error: {e}")))?;

        Ok(items)
    }

    /// LLEN — number of items currently in the buffer (for metrics / logging).
    pub async fn llen(&self, key: &str) -> Result<i64, AppError> {
        let mut conn = self.conn.clone();
        let len: i64 = redis::cmd("LLEN")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Internal(format!("Redis LLEN error: {e}")))?;
        Ok(len)
    }
}
