// src/models/command.rs
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;

/// A motor rotation command — maps to the `roter_commands` table.
#[derive(Debug, Serialize, FromRow)]
pub struct Command {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub source: String,
    pub target_lux_l: Option<i32>,
    pub target_lux_r: Option<i32>,
    pub status: String,
    pub executed_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub lux_left: Option<i32>,
    pub lux_right: Option<i32>,
    pub response_note: Option<String>,
}
