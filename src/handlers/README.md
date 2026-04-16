# Handlers

**What:** HTTP layer — receives requests, calls services, returns responses.
**When:** Add a new handler when you need a new API endpoint.

```rust
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // delegate to service, return JSON
}
```
