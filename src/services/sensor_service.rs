use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use sqlx::PgPool;

use crate::error::AppError;
use crate::models::sensor::SensorLog;
use crate::schemas::sensor::{SensorInsertedResponse, SensorPayload};
use crate::services::redis_service::{Redis, SENSOR_BUFFER_KEY, ttl::SENSOR_BUFFER};

/// Serialize `payload` to JSON and push it onto the Redis sensor buffer.
pub async fn buffer_sensor_reading(redis: &Redis, payload: &SensorPayload) -> Result<(), AppError> {
    let json = serde_json::to_string(payload)
        .map_err(|e| AppError::Internal(format!("JSON serialize error: {e}")))?;
    redis.lpush(SENSOR_BUFFER_KEY, &json, SENSOR_BUFFER).await?;
    Ok(())
}

/// Insert the newest buffered reading and discard older samples from the interval.
pub async fn flush_sensor_buffer(redis: &Redis, pool: &PgPool) -> Result<usize, AppError> {
    let latest_json = match redis.take_latest_and_clear(SENSOR_BUFFER_KEY).await? {
        Some(json) => json,
        None => return Ok(0),
    };

    let payload = match serde_json::from_str::<SensorPayload>(&latest_json) {
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
        VALUES (NOW(), $1, $2, $3, $4, $5, $6, $7, $8, $9, TRUE)
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
    sqlx::query(
        r#"
        INSERT INTO sensor_logs
            (timestamp_slot, lux_l, lux_ml, lux_mr, lux_r, lux_panel_left, lux_panel_right,
             voltage, current, power, is_online)
        VALUES (NOW(), $1, $2, $3, $4, $5, $6, $7, $8, $9, TRUE)
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
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|_| {
        AppError::BadRequest(format!(
            "Invalid date format: '{date_str}'. Expected YYYY-MM-DD"
        ))
    })?;

    let start = Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap());
    let end = start + Duration::days(1);

    let readings = sqlx::query_as::<_, SensorLog>(
        r#"
        SELECT id, timestamp_slot, lux_l, lux_ml, lux_mr, lux_r, lux_panel_left, lux_panel_right,
               voltage, current, power, is_online
        FROM sensor_logs
        WHERE timestamp_slot >= $1 AND timestamp_slot < $2
        ORDER BY timestamp_slot DESC, id DESC
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
    let readings = sqlx::query_as::<_, SensorLog>(
        r#"
        SELECT id, timestamp_slot, lux_l, lux_ml, lux_mr, lux_r, lux_panel_left, lux_panel_right,
               voltage, current, power, is_online
        FROM sensor_logs
        WHERE timestamp_slot >= NOW() - INTERVAL '1 minute'
           OR (id, timestamp_slot) = (
                SELECT id, timestamp_slot
                FROM sensor_logs
                ORDER BY timestamp_slot DESC, id DESC
                LIMIT 1
           )
        ORDER BY timestamp_slot DESC, id DESC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query latest sensor reading: {e}");
        AppError::Internal(format!("Failed to query latest sensor reading: {e}"))
    })?;

    Ok(stabilize_latest_reading(readings, Utc::now()))
}

fn stabilize_latest_reading(mut readings: Vec<SensorLog>, now: DateTime<Utc>) -> Option<SensorLog> {
    readings.sort_by(|left, right| {
        right
            .timestamp_slot
            .cmp(&left.timestamp_slot)
            .then_with(|| right.id.cmp(&left.id))
    });

    let mut latest = readings.first()?.clone();
    latest.lux_l = None;
    latest.lux_ml = None;
    latest.lux_mr = None;
    latest.lux_r = None;
    latest.lux_panel_left = None;
    latest.lux_panel_right = None;
    latest.voltage = None;
    latest.current = None;
    latest.power = None;
    latest.is_online = false;

    let cutoff = now - Duration::minutes(1);
    for reading in readings
        .iter()
        .filter(|reading| reading.timestamp_slot >= cutoff)
    {
        latest.lux_l = latest.lux_l.or(reading.lux_l);
        latest.lux_ml = latest.lux_ml.or(reading.lux_ml);
        latest.lux_mr = latest.lux_mr.or(reading.lux_mr);
        latest.lux_r = latest.lux_r.or(reading.lux_r);
        latest.lux_panel_left = latest.lux_panel_left.or(reading.lux_panel_left);
        latest.lux_panel_right = latest.lux_panel_right.or(reading.lux_panel_right);
        latest.voltage = latest.voltage.or(reading.voltage);
        latest.current = latest.current.or(reading.current);
        latest.power = latest.power.or(reading.power);
        latest.is_online |= reading.is_online;
    }

    Some(latest)
}

pub async fn insert_heartbeat(pool: &PgPool) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO sensor_logs (timestamp_slot, lux_l, lux_ml, lux_mr, lux_r, lux_panel_left, lux_panel_right,
                                 voltage, current, power, is_online)
        VALUES (NOW(), NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, FALSE)
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

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use super::stabilize_latest_reading;
    use crate::models::sensor::SensorLog;

    fn reading(id: i64, age_seconds: i64, panel_left: Option<i32>, online: bool) -> SensorLog {
        SensorLog {
            id,
            timestamp_slot: Utc::now() - Duration::seconds(age_seconds),
            lux_l: None,
            lux_ml: None,
            lux_mr: None,
            lux_r: None,
            lux_panel_left: panel_left,
            lux_panel_right: None,
            voltage: None,
            current: None,
            power: None,
            is_online: online,
        }
    }

    #[test]
    fn test_stabilize_latest_reading_transient_null_retains_recent_value() {
        let now = Utc::now();
        let latest_heartbeat = reading(2, 0, None, false);
        let recent_sensor = reading(1, 30, Some(202), true);

        let latest = stabilize_latest_reading(vec![latest_heartbeat, recent_sensor], now)
            .expect("latest reading should exist");

        assert_eq!(latest.id, 2);
        assert_eq!(latest.lux_panel_left, Some(202));
        assert!(latest.is_online);
    }

    #[test]
    fn test_stabilize_latest_reading_expired_value_returns_null_and_offline() {
        let now = Utc::now();
        let latest_heartbeat = reading(2, 0, None, false);
        let expired_sensor = reading(1, 61, Some(202), true);

        let latest = stabilize_latest_reading(vec![latest_heartbeat, expired_sensor], now)
            .expect("latest reading should exist");

        assert_eq!(latest.lux_panel_left, None);
        assert!(!latest.is_online);
    }
}
