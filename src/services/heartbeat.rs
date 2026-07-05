// src/services/heartbeat.rs
use sqlx::PgPool;
use tokio::time::{self, Duration};

/// Spawn a background task that inserts a heartbeat row every 5 seconds.
/// All fields are NULL — the heartbeat's purpose is to mark that time is passing.
///
/// The task runs until the provided `shutdown` signal fires (server shutting down).
/// Errors are logged as warnings but never crash the server.
pub fn spawn(pool: PgPool, mut shutdown: tokio::sync::watch::Receiver<bool>) {
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(5));
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = crate::services::sensor_service::insert_heartbeat(&pool).await {
                        tracing::warn!("Heartbeat insert failed: {e}");
                    } else {
                        tracing::debug!("Heartbeat inserted");
                    }
                }
                _ = shutdown.changed() => {
                    tracing::info!("Heartbeat scheduler shutting down");
                    break;
                }
            }
        }
    });
}
