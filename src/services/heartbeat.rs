// src/services/heartbeat.rs
//
// Background scheduler — two tasks share a single shutdown signal:
//
//  1. Sensor flusher        — every 60 s: read Redis → UPSERT PostgreSQL (id=0)
//  2. Heartbeat inserter    — every 60 s: insert an all-NULL row so the dashboard
//                              knows the server is alive even between real readings
//
// Both tasks log warnings on error but never crash the server.

use sqlx::PgPool;
use tokio::time::{self, Duration};

use crate::services::redis_service::Redis;
use crate::services::sensor_service;

/// Spawn both background tasks and return immediately.
/// Both tasks stop when `shutdown` transitions to `true`.
pub fn spawn(
    pool: PgPool,
    redis: Option<Redis>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    tokio::spawn(async move {
        // 60 วินาที — sync กับ Redis TTL (SENSOR_CURRENT = 60s)
        let mut interval = time::interval(Duration::from_secs(60));
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // ── 1. Flush current sensor reading (Redis → PostgreSQL) ──
                    if let Some(ref r) = redis {
                        match sensor_service::flush_current_reading(r, &pool).await {
                            Ok(0) => {} // no data in Redis — no log noise
                            Ok(n) => tracing::info!("Flushed {n} current sensor reading → PostgreSQL"),
                            Err(e) => tracing::warn!("Sensor flush failed: {e}"),
                        }
                    }

                    // ── 2. Heartbeat INSERT (all-NULL row) ──
                    if let Err(e) = sensor_service::insert_heartbeat(&pool).await {
                        tracing::warn!("Heartbeat insert failed: {e}");
                    } else {
                        tracing::debug!("Heartbeat inserted");
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
                        tracing::info!("Scheduler shutting down — performing final flush");
                        match sensor_service::flush_current_reading(r, &pool).await {
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
