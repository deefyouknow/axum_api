use chrono::{NaiveDate, Utc, Duration, TimeZone};
use sqlx::PgPool;

use crate::error::AppError;
use crate::models::sensor::SensorLog;
use crate::schemas::sensor::{SensorInsertedResponse, SensorPayload};
use crate::services::redis_service::{
    Redis, SENSOR_CURRENT_KEY, ttl::SENSOR_CURRENT,
};

/// Serialize `payload` to JSON and store as the latest reading in Redis (SET, not LPUSH).
/// TTL = 60 seconds — ถ้าไม่มีใคร post lux มา 1 นาที ถึงจะล้างข้อมูล
pub async fn buffer_sensor_reading(
    redis: &Redis,
    payload: &SensorPayload,
) -> Result<(), AppError> {
    let json = serde_json::to_string(payload)
        .map_err(|e| AppError::Internal(format!("JSON serialize error: {e}")))?;
    redis.set(SENSOR_CURRENT_KEY, &json, SENSOR_CURRENT).await?;
    Ok(())
}

/// Flush the latest sensor reading from Redis → PostgreSQL (UPSERT on fixed ID = 0).
/// อัพเดท row เดิมแทนที่จะสร้าง row ใหม่ทุกครั้ง
pub async fn flush_current_reading(redis: &Redis, pool: &PgPool) -> Result<usize, AppError> {
    let json = match redis.get(SENSOR_CURRENT_KEY).await? {
        Some(j) => j,
        None => return Ok(0), // key expired or never set
    };

    let payload = match serde_json::from_str::<SensorPayload>(&json) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Skipping malformed sensor current reading: {e}");
            return Ok(0);
        }
    };

    sqlx::query(
        r#"
        INSERT INTO sensor_current_reading
            (id, timestamp_slot, lux_l, lux_ml, lux_mr, lux_r, lux_panel_left, lux_panel_right,
             voltage, current, power, is_online)
        VALUES (0, NOW(), $1, $2, $3, $4, $5, $6, $7, $8, $9, TRUE)
        ON CONFLICT (id) DO UPDATE SET
            timestamp_slot = EXCLUDED.timestamp_slot,
            lux_l          = EXCLUDED.lux_l,
            lux_ml         = EXCLUDED.lux_ml,
            lux_mr         = EXCLUDED.lux_mr,
            lux_r          = EXCLUDED.lux_r,
            lux_panel_left = EXCLUDED.lux_panel_left,
            lux_panel_right= EXCLUDED.lux_panel_right,
            voltage        = EXCLUDED.voltage,
            current        = EXCLUDED.current,
            power          = EXCLUDED.power,
            is_online      = TRUE
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
        tracing::error!("UPSERT sensor_current_reading failed: {e}");
        AppError::Internal(format!("UPSERT failed: {e}"))
    })?;

    tracing::debug!("Flushed current sensor reading from Redis → PostgreSQL (UPSERT id=0)");
    Ok(1)
}

pub async fn insert_sensor_reading(
    pool: &PgPool,
    payload: SensorPayload,
) -> Result<SensorInsertedResponse, AppError> {
    // UPSERT ลง sensor_current_reading (id=0) — อัพเดทค่าล่าสุด
    sqlx::query(
        r#"
        INSERT INTO sensor_current_reading
            (id, timestamp_slot, lux_l, lux_ml, lux_mr, lux_r, lux_panel_left, lux_panel_right,
             voltage, current, power, is_online)
        VALUES (0, NOW(), $1, $2, $3, $4, $5, $6, $7, $8, $9, TRUE)
        ON CONFLICT (id) DO UPDATE SET
            timestamp_slot = EXCLUDED.timestamp_slot,
            lux_l          = EXCLUDED.lux_l,
            lux_ml         = EXCLUDED.lux_ml,
            lux_mr         = EXCLUDED.lux_mr,
            lux_r          = EXCLUDED.lux_r,
            lux_panel_left = EXCLUDED.lux_panel_left,
            lux_panel_right= EXCLUDED.lux_panel_right,
            voltage        = EXCLUDED.voltage,
            current        = EXCLUDED.current,
            power          = EXCLUDED.power,
            is_online      = TRUE
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
        tracing::error!("UPSERT sensor_current_reading (slow path) failed: {e}");
        AppError::Internal(format!("UPSERT failed: {e}"))
    })?;

    Ok(SensorInsertedResponse {
        success: true,
        message: "Upserted current reading to DB".to_string(),
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
    // อ่านจาก sensor_current_reading (row เดียว id=0) — ค่าล่าสุดที่เสถียร
    let reading = sqlx::query_as::<_, SensorLog>(
        r#"
        SELECT id::BIGINT, timestamp_slot, lux_l, lux_ml, lux_mr, lux_r, lux_panel_left, lux_panel_right,
               voltage, current, power, is_online
        FROM sensor_current_reading
        WHERE is_online = TRUE
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
