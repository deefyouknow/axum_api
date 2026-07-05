// src/schemas/command.rs
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// POST /commands — create a new command
#[derive(Debug, Deserialize)]
pub struct CreateCommandRequest {
    pub source: Option<String>,       // 'manual' | 'algorithm' (default: 'manual')
    pub target_lux_l: Option<i32>,
    pub target_lux_r: Option<i32>,
}

/// Response for POST /commands
#[derive(Debug, Serialize)]
pub struct CommandResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub source: String,
    pub target_lux_l: Option<i32>,
    pub target_lux_r: Option<i32>,
    pub status: String,
}

/// PATCH /commands/:id/response — ESP32 reports execution result
#[derive(Debug, Deserialize)]
pub struct UpdateCommandRequest {
    pub status: String,               // 'executing' | 'success' | 'failed'
    pub lux_left: Option<i32>,
    pub lux_right: Option<i32>,
    pub response_note: Option<String>,
}

/// Response for PATCH /commands/:id/response
#[derive(Debug, Serialize)]
pub struct UpdateCommandResponse {
    pub id: i64,
    pub status: String,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Query params for GET /commands/history
#[derive(Debug, Deserialize)]
pub struct CommandHistoryQuery {
    pub limit: Option<i64>,           // default: 50
}

/// Response for GET /commands/pending and GET /commands/history
#[derive(Debug, Serialize)]
pub struct CommandListResponse {
    pub commands: Vec<crate::models::command::Command>,
    pub count: usize,
}
