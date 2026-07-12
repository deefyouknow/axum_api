use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;

use axum_api::services::redis_service::Redis;
use axum_api::state::AppState;

#[tokio::main]
async fn main() {
    // ── Init ──────────────────────────────────────────────
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "axum_api=debug,info".into()),
        )
        .init();

    // ── Config ────────────────────────────────────────────
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set in .env");
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "4000".into())
        .parse()
        .expect("PORT must be a valid number");

    // ── Database ──────────────────────────────────────────
    // acquire_timeout: 5 s — fail fast instead of queuing for 30 s.
    //   When DB is slow the ESP32 will get a 503 quickly, retry, and the
    //   Redis buffer will absorb the data in the meantime.
    // min_connections: 2 — keep warm connections; avoids handshake latency
    //   on the first request after an idle period.
    // idle_timeout: 60 s — recycle connections that have been idle too long
    //   (avoids ETIMEDOUT from the DB closing them on its side first).
    // max_lifetime: 1800 s — periodically replace long-lived connections to
    //   pick up fresh settings and avoid silent TCP resets.
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .idle_timeout(std::time::Duration::from_secs(60))
        .max_lifetime(std::time::Duration::from_secs(1800))
        .test_before_acquire(true)
        .connect(&db_url)
        .await
        .expect("Failed to connect to database");

    tracing::info!(
        "Database pool ready (max {} connections)",
        pool.options().get_max_connections()
    );

    // ── Redis (optional) ──────────────────────────────────
    let redis = match std::env::var("REDIS_URL") {
        Ok(url) => match Redis::connect(&url).await {
            Ok(r) => {
                tracing::info!("Redis connected — sensor write-buffer enabled");
                Some(r)
            }
            Err(e) => {
                tracing::warn!("Redis unavailable, running without cache/buffer: {e}");
                None
            }
        },
        Err(_) => {
            tracing::info!("REDIS_URL not set, running without cache/buffer");
            None
        }
    };

    // ── State ─────────────────────────────────────────────
    let state = AppState {
        db: pool.clone(),
        jwt_secret,
        redis: redis.clone(),
    };

    // ── Background scheduler (flush buffer + heartbeat) ───
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    axum_api::services::heartbeat::spawn(pool.clone(), redis, shutdown_rx);

    // ── Routes ────────────────────────────────────────────
    let protected = axum_api::routes::protected_routes()
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            axum_api::middleware::auth::require_auth,
        ));

    let app = axum_api::routes::public_routes()
        .merge(protected)
        .with_state(state);

    // ── Serve ─────────────────────────────────────────────
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::info!("🦀 Server listening on http://{addr}");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal(shutdown_tx))
    .await
    .unwrap();
}

/// Wait for Ctrl+C, then signal all background tasks to stop.
async fn shutdown_signal(shutdown_tx: tokio::sync::watch::Sender<bool>) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, starting graceful shutdown");
    let _ = shutdown_tx.send(true);
}
