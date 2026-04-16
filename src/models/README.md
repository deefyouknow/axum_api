# Models

**What:** Database structs (`FromRow`) and internal data types (e.g. JWT claims).
**When:** Add a new model when you add a new DB table or internal data shape.

```rust
#[derive(FromRow)]
pub struct UserRow {
    pub username: String,
    pub password: String,
}
```
