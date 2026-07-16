use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::models::sensor::SensorLog;

/// POST /api/sensors/reading — payload from ESP32
#[derive(Debug, Serialize, Deserialize)]
pub struct SensorPayload {
    pub lux_panel_left: Option<i32>,
    pub lux_panel_right: Option<i32>,
    pub lux_l: Option<i32>,
    pub lux_ml: Option<i32>,
    pub lux_mr: Option<i32>,
    pub lux_r: Option<i32>,
    pub voltage: Option<f32>,
    pub current: Option<f32>,
    pub power: Option<f32>,
}

#[derive(Debug, Serialize)]
pub struct SensorInsertedResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub date: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SensorHistoryResponse {
    pub readings: Vec<SensorLog>,
    pub date: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct SensorLatestResponse {
    pub reading: Option<SensorLog>,
}
