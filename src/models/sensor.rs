use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct SensorLog {
    pub id: i64,
    pub timestamp_slot: DateTime<Utc>,
    pub lux_l: Option<i32>,
    pub lux_ml: Option<i32>,
    pub lux_mr: Option<i32>,
    pub lux_r: Option<i32>,
    pub lux_panel_left: Option<i32>,
    pub lux_panel_right: Option<i32>,
    pub voltage: Option<f32>,
    pub current: Option<f32>,
    pub power: Option<f32>,
    pub is_online: bool,
}

/// Single-row table for the latest sensor reading (id = 0 always).
/// UPSERT อัพเดทค่าเดิมแทนที่จะสร้าง row ใหม่ทุกครั้ง
#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct SensorCurrentReading {
    pub id: i32,
    pub timestamp_slot: DateTime<Utc>,
    pub lux_l: Option<i32>,
    pub lux_ml: Option<i32>,
    pub lux_mr: Option<i32>,
    pub lux_r: Option<i32>,
    pub lux_panel_left: Option<i32>,
    pub lux_panel_right: Option<i32>,
    pub voltage: Option<f32>,
    pub current: Option<f32>,
    pub power: Option<f32>,
    pub is_online: bool,
}
