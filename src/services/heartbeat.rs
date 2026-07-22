// src/services/heartbeat.rs
//
// Background scheduler — two tasks share a single shutdown signal:
//
//  1. Sensor buffer flusher  — every 5 s: drain Redis → batch INSERT PostgreSQL
//  2. Heartbeat inserter     — every 5 s: insert an all-NULL row so the dashboard
//                              knows the server is alive even between real readings
//
// Both tasks log warnings on error but never crash the server.

use sqlx::PgPool;
use tokio::time::{self, Duration};

use crate::services::redis_service::Redis;
use crate::services::sensor_service;

fn should_insert_heartbeat(flushed_rows: usize) -> bool {
    flushed_rows == 0
}

/// Spawn both background tasks and return immediately.
/// Both tasks stop when `shutdown` transitions to `true`.
pub fn spawn(pool: PgPool, redis: Option<Redis>, mut shutdown: tokio::sync::watch::Receiver<bool>) {
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(5));
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // ── 1. Flush sensor write-buffer (Redis → PostgreSQL) ──
                    let flushed_rows = if let Some(ref r) = redis {
                        match sensor_service::flush_sensor_buffer(r, &pool).await {
                            Ok(n) => {
                                if n > 0 {
                                    tracing::info!("Flushed {n} sensor readings → PostgreSQL");
                                }
                                n
                            }
                            Err(e) => {
                                tracing::warn!("Sensor buffer flush failed: {e}");
                                0
                            }
                        }
                    } else {
                        0
                    };

                    // ── 2. Heartbeat INSERT (all-NULL row) ──
                    if should_insert_heartbeat(flushed_rows) {
                        if let Err(e) = sensor_service::insert_heartbeat(&pool).await {
                            tracing::warn!("Heartbeat insert failed: {e}");
                        } else {
                            tracing::debug!("Heartbeat inserted");
                        }
                    }

                    // ── 3. Auto-cancel stale commands (> 10 mins) ──
                    if let Err(e) = sqlx::query(
                        "UPDATE active_commands SET status = 2, completed_at = NOW() WHERE status = 0 AND created_at < NOW() - INTERVAL '10 minutes'"
                    ).execute(&pool).await {
                        tracing::warn!("Auto-cancel stale commands failed: {e}");
                    }
                }
                _ = shutdown.changed() => {
                    // Final flush before shutting down — don't lose buffered data
                    if let Some(ref r) = redis {
                        tracing::info!("Scheduler shutting down — performing final buffer flush");
                        match sensor_service::flush_sensor_buffer(r, &pool).await {
                            Ok(n) => tracing::info!("Final flush: {n} rows written"),
                            Err(e) => tracing::warn!("Final flush failed: {e}"),
                        }
                    }
                    tracing::info!("Background scheduler stopped");
                    break;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::should_insert_heartbeat;

    #[test]
    fn test_should_insert_heartbeat_sensor_insert_suppresses_heartbeat() {
        assert!(!should_insert_heartbeat(1));
    }

    #[test]
    fn test_should_insert_heartbeat_empty_buffer_requires_heartbeat() {
        assert!(should_insert_heartbeat(0));
    }
}
