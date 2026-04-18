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

---

## Endpoints

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| POST | `/auth/register` | `register()` | สร้าง account ใหม่ + คืน JWT ทันที |
| POST | `/auth/login` | `login()` | ตรวจสอบ credentials + คืน JWT |

---

## curl Examples

> **Base URL:** `http://localhost:4000`  
> Port ตรงกับ `PORT` ใน `.env` (default = 4000)

---

### POST /auth/register

**src/handlers/auth_handler.rs** — `pub async fn register(...)`  
Request body: `{ "username": string, "password": string }`

```sh
# src/routes/mod.rs — route("/auth/register", post(auth_handler::register))
curl -X POST http://localhost:4000/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username": "testuser", "password": "secret123"}'
```

**Response 200 OK — สมัครสำเร็จ:**
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..."
}
```

**Response 400 Bad Request — username ซ้ำ:**
```json
{ "error": "Username already taken" }
```

**Response 400 Bad Request — rate-limit by username (Redis, TTL 60s):**
```json
{ "error": "Please wait before registering again" }
```

**Response 400 Bad Request — rate-limit by IP (Redis, TTL 60s):**
```json
{ "error": "Too many registration attempts" }
```

---

### POST /auth/login

**src/handlers/auth_handler.rs** — `pub async fn login(...)`  
Request body: `{ "username": string, "password": string }`

```sh
# src/routes/mod.rs — route("/auth/login", post(auth_handler::login))
curl -X POST http://localhost:4000/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "testuser", "password": "secret123"}'
```

**Response 200 OK — login สำเร็จ:**
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..."
}
```

**Response 401 Unauthorized — username ไม่มีในระบบ หรือ password ผิด:**
```json
{ "error": "Invalid username or password" }
```

---

### ใช้ Token ที่ได้ (ตัวอย่างสำหรับ endpoint อื่นในอนาคต)

```sh
# เก็บ token ไว้ใน variable
TOKEN=$(curl -s -X POST http://localhost:4000/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "testuser", "password": "secret123"}' \
  | grep -o '"token":"[^"]*"' | cut -d'"' -f4)

# ใช้ token กับ endpoint ที่ต้องการ auth
curl -X GET http://localhost:4000/some/protected/route \
  -H "Authorization: Bearer $TOKEN"
```

---

## Error Reference

| HTTP Status | AppError variant | สาเหตุ |
|-------------|-----------------|--------|
| 400 | `BadRequest` | ข้อมูลไม่ถูกต้อง / username ซ้ำ / rate-limited |
| 401 | `Unauthorized` | credentials ผิด / token หมดอายุ |
| 404 | `NotFound` | ไม่พบ resource |
| 500 | `Internal` | DB error / bcrypt error / JWT error |

> ดู `src/error.rs` สำหรับ error type ทั้งหมด
