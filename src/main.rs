mod error;
mod handlers;
mod models;
mod routes;
mod schemas;
mod services;
mod state;

use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;

use crate::services::redis_service::Redis;
use crate::state::AppState;

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
    let pool = PgPoolOptions::new()
        .max_connections(20) // 1. เพิ่มจำนวนท่อถ้าแรมไหว (5 น้อยไปสำหรับระบบจริง)
        .min_connections(2) // 2. รักษาท่อขั้นต่ำให้ "เปิดค้างไว้" ตลอดเวลาเหมือน Wi-Fi
        .acquire_timeout(std::time::Duration::from_secs(30)) // 3. ให้เวลาชะเง้อรอท่อนานขึ้นหน่อย
        .idle_timeout(std::time::Duration::from_secs(300)) // 4. ลดจาก 10 → 5 นาที ให้ pool ไล่ท่อเก่าทิ้งก่อน firewall ตัด
        .max_lifetime(std::time::Duration::from_secs(1800)) // 5. ล้างท่อใหม่ทุก 30 นาทีป้องกันท่อเสื่อม
        .test_before_acquire(false) // 6. ปิด ping ก่อนใช้ท่อ — ลด 1 round-trip ทุก request (idle_timeout จัดการท่อตายแทน)
        .connect(&db_url) // 7. เชื่อมทันทีตอน start — request แรกไม่ต้องรอ TCP+TLS+PG auth อีกต่อไป
        .await
        .expect("Failed to connect to database");

    tracing::info!("Database pool ready ({} connections)", 2);

    // ── Redis (optional) ──────────────────────────────────
    let redis = match std::env::var("REDIS_URL") {
        Ok(url) => match Redis::connect(&url).await {
            Ok(r) => {
                tracing::info!("Redis connected");
                Some(r)
            }
            Err(e) => {
                tracing::warn!("Redis unavailable, running without cache: {e}");
                None
            }
        },
        Err(_) => {
            tracing::info!("REDIS_URL not set, running without cache");
            None
        }
    };

    // ── State + Routes ────────────────────────────────────
    let state = AppState {
        db: pool,
        jwt_secret,
        redis,
    };

    let app = routes::auth_routes().with_state(state);

    // ── Serve ─────────────────────────────────────────────
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::info!("🦀 Server listening on http://{addr}");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap();
}
