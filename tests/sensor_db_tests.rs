use axum_api::services::sensor_service;
use sqlx::PgPool;

async fn test_pool() -> Option<PgPool> {
    let database_url = std::env::var("DATABASE_URL").ok()?;
    Some(
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&database_url)
            .await
            .expect("test database should connect"),
    )
}

#[tokio::test]
async fn test_get_latest_reading_transient_null_returns_recent_field_value() {
    let Some(pool) = test_pool().await else {
        eprintln!("DATABASE_URL not set; skipping PostgreSQL integration test");
        return;
    };

    let sensor_id: i64 = sqlx::query_scalar(
        "INSERT INTO sensor_logs (timestamp_slot, lux_panel_left, is_online)
         VALUES (NOW() + INTERVAL '30 seconds', 987654321, TRUE)
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("sensor fixture should insert");
    let heartbeat_id: i64 = sqlx::query_scalar(
        "INSERT INTO sensor_logs (timestamp_slot, is_online)
         VALUES (NOW() + INTERVAL '31 seconds', FALSE)
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("heartbeat fixture should insert");

    let latest = sensor_service::get_latest_reading(&pool)
        .await
        .expect("latest query should succeed")
        .expect("latest row should exist");

    assert_eq!(latest.id, heartbeat_id);
    assert_eq!(latest.lux_panel_left, Some(987654321));
    assert!(latest.is_online);

    sqlx::query("DELETE FROM sensor_logs WHERE id = $1 OR id = $2")
        .bind(sensor_id)
        .bind(heartbeat_id)
        .execute(&pool)
        .await
        .expect("fixtures should be cleaned up");
}
