// src/schemas/sensor.rs
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::models::sensor::SensorReading;

/// POST /sensors/reading — single payload from ESP32 (all fields optional).
/// Payload format is unchanged from the old 3-table design for backward compatibility.
/// `Serialize` is required so the payload can be JSON-encoded into the Redis write-buffer.
#[derive(Debug, Serialize, Deserialize)]
pub struct SensorPayload {
    // Solar lux group (BH1750FVI left/right)
    pub lux_left: Option<i32>,
    pub lux_right: Option<i32>,

    // Array lux group (BH1750FVI x4 via TCA9548A)
    pub lux_l: Option<i32>,
    pub lux_ml: Option<i32>,
    pub lux_mr: Option<i32>,
    pub lux_r: Option<i32>,

    // Power monitor group (INA219) + limit switches
    pub ina_voltage: Option<i32>,   // millivolts
    pub ina_current: Option<i32>,   // milliamps
    pub ina_power: Option<i32>,     // milliwatts
    pub limit_sw_left: Option<bool>,
    pub limit_sw_right: Option<bool>,
}

/// Response for POST /sensors/reading — now returns single row id + time.
#[derive(Debug, Serialize)]
pub struct SensorInsertedResponse {
    pub success: bool,
    pub id: Option<i64>,
    pub time: Option<DateTime<Utc>>,
}

/// Query params for GET /sensors/history
#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    /// Date in YYYY-MM-DD format (e.g. "2026-07-05")
    pub date: Option<String>,
    /// Optional limit (default: 1000)
    pub limit: Option<i64>,
}

/// Response for GET /sensors/history
#[derive(Debug, Serialize)]
pub struct SensorHistoryResponse {
    pub readings: Vec<SensorReading>,
    pub date: String,
    pub count: usize,
}

/// Response for GET /sensors/latest
#[derive(Debug, Serialize)]
pub struct SensorLatestResponse {
    pub reading: Option<SensorReading>,
}
