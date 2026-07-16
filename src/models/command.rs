use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct ActiveCommand {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub function_name: String,
    pub from_user: String,
    pub target_type: String,
    pub target_value: Option<f32>,
    pub target_left_ratio: Option<f32>,
    pub target_right_ratio: Option<f32>,
    pub tolerance: f32,
    pub lux_left: Option<i32>,
    pub lux_right: Option<i32>,
    pub status: i16,
}
