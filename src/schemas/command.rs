use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::models::command::ActiveCommand;

#[derive(Debug, Deserialize)]
pub struct CreateCommandRequest {
    pub from_user: String, // 'dashboard' | 'ML_AI'
    pub target_type: String, // 'error' | 'light_bias'
    pub target_value: Option<f32>,
    pub target_left_ratio: Option<f32>,
    pub target_right_ratio: Option<f32>,
    pub tolerance: f32,
}

#[derive(Debug, Serialize)]
pub struct CommandResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub status: i16,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCommandRequest {
    pub status: Option<i16>,
    pub lux_left: Option<i32>,
    pub lux_right: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct UpdateCommandResponse {
    pub id: i64,
    pub status: i16,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CommandHistoryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub status: Option<i16>,
    pub from_user: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CommandListResponse {
    pub commands: Vec<ActiveCommand>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct CommandSingleResponse {
    pub command: Option<ActiveCommand>,
}
