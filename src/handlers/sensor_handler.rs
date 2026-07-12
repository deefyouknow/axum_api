// src/handlers/sensor_handler.rs
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use crate::{
    error::AppError,
    schemas::sensor::{
        HistoryQuery, SensorHistoryResponse, SensorInsertedResponse, SensorLatestResponse,
        SensorPayload,
    },
    services::sensor_service,
    state::AppState,
};

/// POST /sensors/reading
///
/// **Fast path (Redis available):**  
/// Pushes the payload onto the Redis write-buffer and returns 201 immediately
/// (~1 ms). The background flusher drains the buffer into PostgreSQL every 5 s.
///
/// **Slow path (Redis unavailable):**  
/// Falls back to a direct INSERT into PostgreSQL (original behaviour).
pub async fn post_sensor_reading(
    State(state): State<AppState>,
    Json(payload): Json<SensorPayload>,
) -> Result<(StatusCode, Json<SensorInsertedResponse>), AppError> {
    if let Some(ref redis) = state.redis {
        // Fast path — buffer in Redis, respond immediately
        sensor_service::buffer_sensor_reading(redis, &payload).await?;

        let resp = SensorInsertedResponse {
            success: true,
            id: None,   // id is assigned by PostgreSQL on flush
            time: None, // likewise — not yet persisted
        };
        return Ok((StatusCode::CREATED, Json(resp)));
    }

    // Slow path — direct INSERT (Redis not configured)
    let result = sensor_service::insert_sensor_reading(&state.db, payload).await?;
    Ok((StatusCode::CREATED, Json(result)))
}

/// GET /sensors/history?date=2026-07-05&limit=1000
/// Returns all sensor readings for a specific date.
pub async fn get_sensor_history(
    State(state): State<AppState>,
    Query(params): Query<HistoryQuery>,
) -> Result<Json<SensorHistoryResponse>, AppError> {
    let date_str = params.date.unwrap_or_else(|| {
        chrono::Utc::now().format("%Y-%m-%d").to_string()
    });
    let limit = params.limit.unwrap_or(1000);

    let readings = sensor_service::get_history_by_date(&state.db, &date_str, limit).await?;
    let count = readings.len();

    Ok(Json(SensorHistoryResponse {
        readings,
        date: date_str,
        count,
    }))
}

/// GET /sensors/latest
/// Returns the most recent sensor reading.
pub async fn get_sensor_latest(
    State(state): State<AppState>,
) -> Result<Json<SensorLatestResponse>, AppError> {
    let reading = sensor_service::get_latest_reading(&state.db).await?;
    Ok(Json(SensorLatestResponse { reading }))
}
