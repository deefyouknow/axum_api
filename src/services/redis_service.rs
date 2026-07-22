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

/// Redis list key used as the sensor write-buffer.
pub const SENSOR_BUFFER_KEY: &str = "sensor:buffer";

fn latest_buffer_index() -> i64 {
    0
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

    /// Atomically return the newest LPUSH item and clear the interval backlog.
    pub async fn take_latest_and_clear(&self, key: &str) -> Result<Option<String>, AppError> {
        let mut conn = self.conn.clone();
        let script = redis::Script::new(
            r#"
            local latest = redis.call('LINDEX', KEYS[1], ARGV[1])
            if latest then
                redis.call('DEL', KEYS[1])
            end
            return latest
            "#,
        );

        script
            .key(key)
            .arg(latest_buffer_index())
            .invoke_async(&mut conn)
            .await
            .map_err(|e| AppError::Internal(format!("Redis take latest error: {e}")))
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

#[cfg(test)]
mod tests {
    use super::latest_buffer_index;

    #[test]
    fn test_latest_buffer_index_lpush_order_returns_head() {
        assert_eq!(latest_buffer_index(), 0);
    }
}
