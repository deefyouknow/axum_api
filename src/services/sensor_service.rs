use chrono::{NaiveDate, Utc, Duration, TimeZone};
use sqlx::PgPool;

use crate::error::AppError;
use crate::models::sensor::SensorLog;
use crate::schemas::sensor::{SensorInsertedResponse, SensorPayload};
use crate::services::redis_service::{
    Redis, SENSOR_BUFFER_BATCH, SENSOR_BUFFER_KEY, ttl::SENSOR_BUFFER,
};

/// Serialize `payload` to JSON and push it onto the Redis sensor buffer.
pub async fn buffer_sensor_reading(
    redis: &Redis,
    payload: &SensorPayload,
) -> Result<(), AppError> {
    let json = serde_json::to_string(payload)
        .map_err(|e| AppError::Internal(format!("JSON serialize error: {e}")))?;
    redis.lpush(SENSOR_BUFFER_KEY, &json, SENSOR_BUFFER).await?;
    Ok(())
}

/// We change the behavior to only get the *latest* reading from Redis (last-value)
/// and insert it. Wait, the implementation plan says:
/// "ดึงค่าล่าสุด (Last-Value) จาก Redis มา INSERT ลงตาราง sensor_logs"
/// To do this, we can just pop the first element (the latest) and ignore the rest,
/// or get all elements and take the first, then clear the list.
pub async fn flush_sensor_buffer(redis: &Redis, pool: &PgPool) -> Result<usize, AppError> {
    // Get all items in the buffer, we only care about the latest one
    let items = redis.lrange_and_trim(SENSOR_BUFFER_KEY, SENSOR_BUFFER_BATCH).await?;

    if items.is_empty() {
        return Ok(0);
    }

    // The first item is the most recent (because we used lpush)
    let latest_json = &items[0];
    
    let payload = match serde_json::from_str::<SensorPayload>(latest_json) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Skipping malformed sensor buffer entry: {e}");
            return Ok(0);
        }
    };

    sqlx::query(
        r#"
        INSERT INTO sensor_logs
            (timestamp_slot, lux_l, lux_ml, lux_mr, lux_r, lux_panel_left, lux_panel_right,
             voltage, current, power, is_online)
        VALUES (date_trunc('minute', NOW()), $1, $2, $3, $4, $5, $6, $7, $8, $9, TRUE)
        "#,
    )
    .bind(payload.lux_l)
    .bind(payload.lux_ml)
    .bind(payload.lux_mr)
    .bind(payload.lux_r)
    .bind(payload.lux_panel_left)
    .bind(payload.lux_panel_right)
    .bind(payload.voltage)
    .bind(payload.current)
    .bind(payload.power)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("INSERT sensor_logs failed: {e}");
        AppError::Internal(format!("INSERT failed: {e}"))
    })?;

    tracing::debug!("Flushed latest sensor reading from Redis → PostgreSQL");
    Ok(1)
}

pub async fn insert_sensor_reading(
    pool: &PgPool,
    payload: SensorPayload,
) -> Result<SensorInsertedResponse, AppError> {
    let row = sqlx::query_as::<_, SensorLog>(
        r#"
        INSERT INTO sensor_logs
            (timestamp_slot, lux_l, lux_ml, lux_mr, lux_r, lux_panel_left, lux_panel_right,
             voltage, current, power, is_online)
        VALUES (date_trunc('minute', NOW()), $1, $2, $3, $4, $5, $6, $7, $8, $9, TRUE)
        RETURNING id, timestamp_slot, lux_l, lux_ml, lux_mr, lux_r, lux_panel_left, lux_panel_right,
                  voltage, current, power, is_online
        "#,
    )
    .bind(payload.lux_l)
    .bind(payload.lux_ml)
    .bind(payload.lux_mr)
    .bind(payload.lux_r)
    .bind(payload.lux_panel_left)
    .bind(payload.lux_panel_right)
    .bind(payload.voltage)
    .bind(payload.current)
    .bind(payload.power)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to insert sensor reading: {e}");
        AppError::Internal(format!("Failed to insert sensor reading: {e}"))
    })?;

    Ok(SensorInsertedResponse {
        success: true,
        message: "Inserted directly to DB".to_string(),
    })
}

pub async fn get_history_by_date(
    pool: &PgPool,
    date_str: &str,
    limit: i64,
) -> Result<Vec<SensorLog>, AppError> {
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest(format!("Invalid date format: '{date_str}'. Expected YYYY-MM-DD")))?;

    let start = Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap());
    let end = start + Duration::days(1);

    let readings = sqlx::query_as::<_, SensorLog>(
        r#"
        SELECT id, timestamp_slot, lux_l, lux_ml, lux_mr, lux_r, lux_panel_left, lux_panel_right,
               voltage, current, power, is_online
        FROM sensor_logs
        WHERE timestamp_slot >= $1 AND timestamp_slot < $2
        ORDER BY timestamp_slot DESC
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

pub async fn get_latest_reading(pool: &PgPool) -> Result<Option<SensorLog>, AppError> {
    let reading = sqlx::query_as::<_, SensorLog>(
        r#"
        SELECT id, timestamp_slot, lux_l, lux_ml, lux_mr, lux_r, lux_panel_left, lux_panel_right,
               voltage, current, power, is_online
        FROM sensor_logs
        ORDER BY timestamp_slot DESC
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

pub async fn insert_heartbeat(pool: &PgPool) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO sensor_logs (timestamp_slot, lux_l, lux_ml, lux_mr, lux_r, lux_panel_left, lux_panel_right,
                                 voltage, current, power, is_online)
        VALUES (date_trunc('minute', NOW()), NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, FALSE)
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

pub async fn get_available_dates(pool: &PgPool) -> Result<Vec<String>, AppError> {
    let dates = sqlx::query_scalar::<_, String>(
        r#"
        SELECT DISTINCT to_char(timestamp_slot, 'YYYY-MM-DD') as date
        FROM sensor_logs
        ORDER BY date DESC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query available dates: {e}");
        AppError::Internal(format!("Failed to query available dates: {e}"))
    })?;

    Ok(dates)
}
