// src/services/sensor_service.rs
use chrono::{NaiveDate, Utc, Duration, TimeZone};
use sqlx::PgPool;

use crate::error::AppError;
use crate::models::sensor::SensorReading;
use crate::schemas::sensor::{SensorInsertedResponse, SensorPayload};
use crate::services::redis_service::{
    Redis, SENSOR_BUFFER_BATCH, SENSOR_BUFFER_KEY, ttl::SENSOR_BUFFER,
};

// ── Redis write-buffer helpers ────────────────────────────────────────────────

/// Serialize `payload` to JSON and push it onto the Redis sensor buffer.
/// Returns immediately (~1 ms) — the actual INSERT happens in `flush_sensor_buffer`.
pub async fn buffer_sensor_reading(
    redis: &Redis,
    payload: &SensorPayload,
) -> Result<(), AppError> {
    let json = serde_json::to_string(payload)
        .map_err(|e| AppError::Internal(format!("JSON serialize error: {e}")))?;
    redis.lpush(SENSOR_BUFFER_KEY, &json, SENSOR_BUFFER).await?;
    Ok(())
}

/// Pull up to `SENSOR_BUFFER_BATCH` entries from Redis and INSERT them into
/// PostgreSQL in a single `UNNEST`-based batch query.
///
/// Called by the background scheduler every 5 seconds.
/// Safe to call even when the buffer is empty — returns immediately.
pub async fn flush_sensor_buffer(redis: &Redis, pool: &PgPool) -> Result<usize, AppError> {
    let items = redis
        .lrange_and_trim(SENSOR_BUFFER_KEY, SENSOR_BUFFER_BATCH)
        .await?;

    if items.is_empty() {
        return Ok(0);
    }

    // Deserialize all items; skip malformed ones with a warning.
    let payloads: Vec<SensorPayload> = items
        .iter()
        .filter_map(|s| match serde_json::from_str::<SensorPayload>(s) {
            Ok(p) => Some(p),
            Err(e) => {
                tracing::warn!("Skipping malformed sensor buffer entry: {e}");
                None
            }
        })
        .collect();

    if payloads.is_empty() {
        return Ok(0);
    }

    let n = payloads.len();

    // Build UNNEST arrays — Option<i32>/Option<bool> map to NULL naturally via sqlx.
    let lux_left:   Vec<Option<i32>>  = payloads.iter().map(|p| p.lux_left).collect();
    let lux_right:  Vec<Option<i32>>  = payloads.iter().map(|p| p.lux_right).collect();
    let lux_l:      Vec<Option<i32>>  = payloads.iter().map(|p| p.lux_l).collect();
    let lux_ml:     Vec<Option<i32>>  = payloads.iter().map(|p| p.lux_ml).collect();
    let lux_mr:     Vec<Option<i32>>  = payloads.iter().map(|p| p.lux_mr).collect();
    let lux_r:      Vec<Option<i32>>  = payloads.iter().map(|p| p.lux_r).collect();
    let roter:      Vec<Option<i32>>  = payloads.iter().map(|p| p.roter_angle).collect();
    let sw_left:    Vec<Option<bool>> = payloads.iter().map(|p| p.limit_sw_left).collect();
    let sw_right:   Vec<Option<bool>> = payloads.iter().map(|p| p.limit_sw_right).collect();

    sqlx::query(
        r#"
        INSERT INTO sensor_readings
            (lux_left, lux_right, lux_l, lux_ml, lux_mr, lux_r,
             roter_angle, limit_sw_left, limit_sw_right)
        SELECT * FROM UNNEST(
            $1::int4[], $2::int4[], $3::int4[], $4::int4[],
            $5::int4[], $6::int4[], $7::int4[],
            $8::bool[], $9::bool[]
        )
        "#,
    )
    .bind(&lux_left)
    .bind(&lux_right)
    .bind(&lux_l)
    .bind(&lux_ml)
    .bind(&lux_mr)
    .bind(&lux_r)
    .bind(&roter)
    .bind(&sw_left)
    .bind(&sw_right)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Batch INSERT sensor_readings failed: {e}");
        AppError::Internal(format!("Batch INSERT failed: {e}"))
    })?;

    tracing::debug!("Flushed {n} sensor readings from Redis → PostgreSQL");
    Ok(n)
}

// ── Direct DB write (fallback when Redis unavailable) ─────────────────────────

/// Insert a single sensor reading into the unified `sensor_readings` table.
/// All fields are nullable — only non-null fields are stored.
pub async fn insert_sensor_reading(
    pool: &PgPool,
    payload: SensorPayload,
) -> Result<SensorInsertedResponse, AppError> {
    let row = sqlx::query_as::<_, SensorReading>(
        r#"
        INSERT INTO sensor_readings
            (lux_left, lux_right, lux_l, lux_ml, lux_mr, lux_r,
             roter_angle, limit_sw_left, limit_sw_right)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, time, lux_left, lux_right, lux_l, lux_ml, lux_mr, lux_r,
                  roter_angle, limit_sw_left, limit_sw_right
        "#,
    )
    .bind(payload.lux_left)
    .bind(payload.lux_right)
    .bind(payload.lux_l)
    .bind(payload.lux_ml)
    .bind(payload.lux_mr)
    .bind(payload.lux_r)
    .bind(payload.roter_angle)
    .bind(payload.limit_sw_left)
    .bind(payload.limit_sw_right)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to insert sensor reading: {e}");
        AppError::Internal(format!("Failed to insert sensor reading: {e}"))
    })?;

    Ok(SensorInsertedResponse {
        success: true,
        id: Some(row.id),
        time: Some(row.time),
    })
}

// ── Query helpers ─────────────────────────────────────────────────────────────

/// Query sensor readings for a specific date (YYYY-MM-DD).
/// Uses partition-aware query: WHERE time >= date AND time < date+1
pub async fn get_history_by_date(
    pool: &PgPool,
    date_str: &str,
    limit: i64,
) -> Result<Vec<SensorReading>, AppError> {
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest(format!("Invalid date format: '{date_str}'. Expected YYYY-MM-DD")))?;

    let start = Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap());
    let end = start + Duration::days(1);

    let readings = sqlx::query_as::<_, SensorReading>(
        r#"
        SELECT id, time, lux_left, lux_right, lux_l, lux_ml, lux_mr, lux_r,
               roter_angle, limit_sw_left, limit_sw_right
        FROM sensor_readings
        WHERE time >= $1 AND time < $2
        ORDER BY time DESC
        LIMIT $3
        "#,
    )
    .bind(start)
    .bind(end)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query sensor history: {e}");
        AppError::Internal(format!("Failed to query sensor history: {e}"))
    })?;

    Ok(readings)
}

/// Get the most recent sensor reading (latest row across all partitions).
pub async fn get_latest_reading(pool: &PgPool) -> Result<Option<SensorReading>, AppError> {
    let reading = sqlx::query_as::<_, SensorReading>(
        r#"
        SELECT id, time, lux_left, lux_right, lux_l, lux_ml, lux_mr, lux_r,
               roter_angle, limit_sw_left, limit_sw_right
        FROM sensor_readings
        ORDER BY time DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query latest sensor reading: {e}");
        AppError::Internal(format!("Failed to query latest sensor reading: {e}"))
    })?;

    Ok(reading)
}

/// Insert a heartbeat row (all null fields) — called by the scheduler every 5s.
pub async fn insert_heartbeat(pool: &PgPool) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO sensor_readings (lux_left, lux_right, lux_l, lux_ml, lux_mr, lux_r,
                                     roter_angle, limit_sw_left, limit_sw_right)
        VALUES (NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to insert heartbeat: {e}");
        AppError::Internal(format!("Failed to insert heartbeat: {e}"))
    })?;

    Ok(())
}
