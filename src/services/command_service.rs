// src/services/command_service.rs
use chrono::Utc;
use sqlx::PgPool;

use crate::error::AppError;
use crate::models::command::Command;
use crate::schemas::command::{
    CommandResponse, CreateCommandRequest, UpdateCommandRequest, UpdateCommandResponse,
};

/// Insert a new command into `roter_commands`.
pub async fn insert_command(
    pool: &PgPool,
    payload: CreateCommandRequest,
) -> Result<CommandResponse, AppError> {
    let source = payload.source.unwrap_or_else(|| "manual".to_string());

    let row = sqlx::query_as::<_, Command>(
        r#"
        INSERT INTO roter_commands (source, target_lux_l, target_lux_r)
        VALUES ($1, $2, $3)
        RETURNING id, created_at, source, target_lux_l, target_lux_r, status,
                  executed_at, completed_at, lux_left, lux_right, response_note
        "#,
    )
    .bind(&source)
    .bind(payload.target_lux_l)
    .bind(payload.target_lux_r)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to insert command: {e}");
        AppError::Internal(format!("Failed to insert command: {e}"))
    })?;

    Ok(CommandResponse {
        id: row.id,
        created_at: row.created_at,
        source: row.source,
        target_lux_l: row.target_lux_l,
        target_lux_r: row.target_lux_r,
        status: row.status,
    })
}

/// Get all commands with status = 'pending', ordered by created_at ASC.
pub async fn get_pending_commands(pool: &PgPool) -> Result<Vec<Command>, AppError> {
    let commands = sqlx::query_as::<_, Command>(
        r#"
        SELECT id, created_at, source, target_lux_l, target_lux_r, status,
               executed_at, completed_at, lux_left, lux_right, response_note
        FROM roter_commands
        WHERE status = 'pending'
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query pending commands: {e}");
        AppError::Internal(format!("Failed to query pending commands: {e}"))
    })?;

    Ok(commands)
}

/// Update command response from ESP32.
pub async fn update_command_response(
    pool: &PgPool,
    command_id: i64,
    payload: UpdateCommandRequest,
) -> Result<UpdateCommandResponse, AppError> {
    let now = Utc::now();

    let row = sqlx::query_as::<_, Command>(
        r#"
        UPDATE roter_commands
        SET status = $2,
            lux_left = $3,
            lux_right = $4,
            response_note = $5,
            executed_at = CASE WHEN $2 = 'executing' THEN $6 ELSE executed_at END,
            completed_at = CASE WHEN $2 IN ('success', 'failed') THEN $6 ELSE completed_at END
        WHERE id = $1
        RETURNING id, created_at, source, target_lux_l, target_lux_r, status,
                  executed_at, completed_at, lux_left, lux_right, response_note
        "#,
    )
    .bind(command_id)
    .bind(&payload.status)
    .bind(payload.lux_left)
    .bind(payload.lux_right)
    .bind(&payload.response_note)
    .bind(now)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update command {command_id}: {e}");
        AppError::Internal(format!("Failed to update command: {e}"))
    })?;

    match row {
        Some(r) => Ok(UpdateCommandResponse {
            id: r.id,
            status: r.status,
            completed_at: r.completed_at,
        }),
        None => Err(AppError::NotFound(format!(
            "Command with id {command_id} not found"
        ))),
    }
}

/// Get command history, ordered by created_at DESC with limit.
pub async fn get_command_history(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<Command>, AppError> {
    let commands = sqlx::query_as::<_, Command>(
        r#"
        SELECT id, created_at, source, target_lux_l, target_lux_r, status,
               executed_at, completed_at, lux_left, lux_right, response_note
        FROM roter_commands
        ORDER BY created_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query command history: {e}");
        AppError::Internal(format!("Failed to query command history: {e}"))
    })?;

    Ok(commands)
}
