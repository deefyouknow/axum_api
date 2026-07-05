// src/handlers/command_handler.rs
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use crate::{
    error::AppError,
    schemas::command::{
        CommandHistoryQuery, CommandListResponse, CommandResponse, CreateCommandRequest,
        UpdateCommandRequest, UpdateCommandResponse,
    },
    services::command_service,
    state::AppState,
};

/// POST /commands — create a new command
pub async fn create_command(
    State(state): State<AppState>,
    Json(payload): Json<CreateCommandRequest>,
) -> Result<(StatusCode, Json<CommandResponse>), AppError> {
    let result = command_service::insert_command(&state.db, payload).await?;
    Ok((StatusCode::CREATED, Json(result)))
}

/// GET /commands/pending — ESP32 polls for pending commands
pub async fn get_pending_commands(
    State(state): State<AppState>,
) -> Result<Json<CommandListResponse>, AppError> {
    let commands = command_service::get_pending_commands(&state.db).await?;
    let count = commands.len();
    Ok(Json(CommandListResponse { commands, count }))
}

/// PATCH /commands/:id/response — ESP32 reports execution result
pub async fn update_command_response(
    State(state): State<AppState>,
    Path(command_id): Path<i64>,
    Json(payload): Json<UpdateCommandRequest>,
) -> Result<Json<UpdateCommandResponse>, AppError> {
    let result =
        command_service::update_command_response(&state.db, command_id, payload).await?;
    Ok(Json(result))
}

/// GET /commands/history?limit=50 — view command history
pub async fn get_command_history(
    State(state): State<AppState>,
    Query(params): Query<CommandHistoryQuery>,
) -> Result<Json<CommandListResponse>, AppError> {
    let limit = params.limit.unwrap_or(50);
    let commands = command_service::get_command_history(&state.db, limit).await?;
    let count = commands.len();
    Ok(Json(CommandListResponse { commands, count }))
}
