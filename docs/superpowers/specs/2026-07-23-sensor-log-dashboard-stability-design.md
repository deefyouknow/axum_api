# Sensor Log and Dashboard Stability Design

## Goal

Record exactly one sensor snapshot every five-second scheduler interval. When the ESP32 has sent data since the previous interval, store the newest payload with its optional fields unchanged. When no payload is available, store an all-`NULL` offline log row. Keep the dashboard stable across short gaps by returning the most recent non-`NULL` value for each sensor field for up to one minute.

The existing HTTP paths, authentication, request JSON, response JSON, status codes, database schema, and documented cURL payloads must not change.

## Root Cause

The current scheduler always inserts a heartbeat after flushing a buffered sensor reading. A successful interval therefore produces both a data row and an all-`NULL` row. Because timestamps are truncated to the minute and latest-row ordering has no deterministic tie-breaker, dashboard reads can alternate between those rows.

The Redis buffer also stores newest readings at the head with `LPUSH`, but its drain operation reads from the tail and treats the first returned element as newest. It can consequently insert an older payload while discarding newer payloads from the same batch.

## Data Flow

1. `POST /sensors/reading` accepts the existing `SensorPayload` and writes it to the Redis buffer when Redis is available. The endpoint contract remains unchanged.
2. Every five seconds, the scheduler atomically consumes the newest buffered payload and clears older payloads that belong to that interval.
3. If a valid payload was consumed, the scheduler inserts exactly one online `sensor_logs` row containing that payload, including any optional `NULL` fields.
4. If the buffer is empty, the scheduler inserts exactly one offline, all-`NULL` heartbeat row.
5. If Redis is unavailable, the existing direct database fallback remains available so ESP32 requests are not rejected. The scheduler continues writing heartbeat rows to preserve periodic liveness logs.

## Dashboard Read Semantics

`GET /sensors/latest` keeps its existing response type. Internally, the service constructs the returned reading as follows:

- Use the newest log row as the response identity and timing reference, ordered by `timestamp_slot DESC, id DESC`.
- For every sensor field independently, return its newest non-`NULL` value whose log timestamp is within the previous one minute.
- A field becomes `NULL` only when it has had no non-`NULL` value during that one-minute window.
- Set `is_online` according to whether a real ESP32 reading exists within the same one-minute window. Heartbeat rows do not by themselves make the ESP32 online.
- Once the one-minute window expires without ESP32 data, the response remains structurally identical but contains `NULL` sensor fields and `is_online: false`.

This per-field rule supports partial ESP32 payloads: one sensor may update while another temporarily sends `NULL`, without making the dashboard flicker.

`GET /sensors/history` remains a raw log view and continues returning the stored five-second snapshots. Its ordering becomes deterministic with `id DESC` as a tie-breaker, without changing the response shape.

## Error Handling and Data Safety

- Redis consumption must be atomic so concurrent ESP32 writes are not accidentally deleted.
- A malformed newest buffer entry is logged and treated as unavailable for that interval; the scheduler writes the offline heartbeat rather than crashing.
- A database insert failure is propagated as `AppError` and logged by the scheduler.
- No production `.unwrap()` calls are introduced.
- No migration or dependency is required.

## Testing

- Unit-test newest-item selection from the Redis list ordering.
- Unit-test scheduler decision logic: successful sensor flush suppresses heartbeat; empty or malformed input selects heartbeat.
- Test the one-minute per-field fallback boundary, including partial payloads and expiry to `NULL`.
- Add contract serialization tests proving the sensor request and response field names are unchanged.
- Update stale sensor integration tests to the already-established contract without changing production schemas.
- Run `cargo fmt --check`, `cargo build`, and `cargo test --all-targets` before completion.

## Out of Scope

- Changing database tables or migrations.
- Renaming sensor fields or changing units.
- Changing endpoint paths, cURL examples, authentication, status codes, or JSON structures.
- Modifying dashboard/frontend code.
