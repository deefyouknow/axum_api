# Routes

**What:** Groups endpoints into logical sets and mounts them on the router.
**When:** Add a new route group when you add a new feature area.

```rust
pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(auth_handler::login))
}
```
