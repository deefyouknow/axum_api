# Services

**What:** Business logic — password hashing, JWT, database queries.
**When:** Add a new service when you need reusable logic outside of HTTP.

```rust
let token = auth_service::generate_jwt(&username, &secret)?;
let user = auth_service::find_user_by_username(&pool, "bob").await?;
```
