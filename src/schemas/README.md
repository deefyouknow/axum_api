# Schemas

**What:** Request/response shapes for the HTTP layer (no DB logic).
**When:** Add a new schema when you need a new API input or output shape.

```rust
#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}
```
