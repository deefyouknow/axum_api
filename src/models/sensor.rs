// src/models/sensor.rs
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;

/// Single sensor reading row — maps to the `sensor_readings` partitioned table.
/// All sensor fields are nullable (ESP32 may not send every field every cycle).
#[derive(Debug, Serialize, FromRow)]
pub struct SensorReading {
    pub id: i64,
    pub time: DateTime<Utc>,
    pub lux_left: Option<i32>,
    pub lux_right: Option<i32>,
    pub lux_l: Option<i32>,
    pub lux_ml: Option<i32>,
    pub lux_mr: Option<i32>,
    pub lux_r: Option<i32>,
    pub roter_angle: Option<i32>,
    pub limit_sw_left: Option<bool>,
    pub limit_sw_right: Option<bool>,
}
