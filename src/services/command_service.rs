use chrono::Utc;
use sqlx::PgPool;

use crate::error::AppError;
use crate::models::command::ActiveCommand;
use crate::schemas::command::{
    CommandResponse, CreateCommandRequest, UpdateCommandRequest, UpdateCommandResponse,
};

pub async fn insert_command(
    pool: &PgPool,
    payload: CreateCommandRequest,
) -> Result<CommandResponse, AppError> {
    let row = sqlx::query_as::<_, ActiveCommand>(
        r#"
        INSERT INTO active_commands (from_user, target_type, target_value, target_left_ratio, target_right_ratio, tolerance)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, created_at, completed_at, function_name, from_user, target_type, target_value, target_left_ratio, target_right_ratio, tolerance, lux_left, lux_right, status
        "#,
    )
    .bind(payload.from_user)
    .bind(payload.target_type)
    .bind(payload.target_value)
    .bind(payload.target_left_ratio)
    .bind(payload.target_right_ratio)
    .bind(payload.tolerance)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to insert command: {e}");
        AppError::Internal(format!("Failed to insert command: {e}"))
    })?;

    Ok(CommandResponse {
        id: row.id,
        created_at: row.created_at,
        status: row.status,
    })
}

pub async fn get_pending_commands(pool: &PgPool) -> Result<Vec<ActiveCommand>, AppError> {
    let commands = sqlx::query_as::<_, ActiveCommand>(
        r#"
        SELECT id, created_at, completed_at, function_name, from_user, target_type, target_value, target_left_ratio, target_right_ratio, tolerance, lux_left, lux_right, status
        FROM active_commands
        WHERE status = 0
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

pub async fn update_command_response(
    pool: &PgPool,
    command_id: i64,
    payload: UpdateCommandRequest,
) -> Result<UpdateCommandResponse, AppError> {
    let now = Utc::now();

    let row = sqlx::query_as::<_, ActiveCommand>(
        r#"
        UPDATE active_commands
        SET status = 1,
            lux_left = $2,
            lux_right = $3,
            completed_at = $4
        WHERE id = $1
        RETURNING id, created_at, completed_at, function_name, from_user, target_type, target_value, target_left_ratio, target_right_ratio, tolerance, lux_left, lux_right, status
        "#,
    )
    .bind(command_id)
    .bind(payload.lux_left)
    .bind(payload.lux_right)
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

pub async fn get_command_history(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<ActiveCommand>, AppError> {
    let commands = sqlx::query_as::<_, ActiveCommand>(
        r#"
        SELECT id, created_at, completed_at, function_name, from_user, target_type, target_value, target_left_ratio, target_right_ratio, tolerance, lux_left, lux_right, status
        FROM active_commands
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
