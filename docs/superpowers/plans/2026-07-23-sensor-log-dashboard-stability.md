# Sensor Log and Dashboard Stability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Insert one sensor snapshot per five-second interval and keep dashboard fields stable through transient `NULL` values for one minute without changing any HTTP contract.

**Architecture:** Redis remains the latest-payload buffer, but an atomic Lua operation will take the head item and clear the interval backlog. The scheduler will insert a heartbeat only when no valid sensor row was inserted. PostgreSQL remains the source for dashboard reads; `/sensors/latest` will assemble each field from its most recent non-`NULL` value inside a one-minute window.

**Tech Stack:** Rust 2024, Axum 0.8, Tokio, Redis 0.26, SQLx 0.8, PostgreSQL, Serde

---

## File Map

- Modify `src/services/redis_service.rs`: atomically consume the newest buffered payload.
- Modify `src/services/sensor_service.rs`: parse the newest payload, insert exact timestamps, and query one-minute per-field fallback values.
- Modify `src/services/heartbeat.rs`: choose exactly one sensor-log insert per scheduler interval.
- Modify `tests/sensor_tests.rs`: replace stale pre-refactor fixtures and test current sensor behavior.
- Modify `tests/common/mod.rs`: clean up `sensor_logs` rather than removed legacy tables.
- Modify `tests/integration_test.rs`: retain and assert current GET/POST contracts.
- Modify `CHANGELOG.md`: document internal behavior changes only.

### Task 1: Atomically Select the Newest ESP32 Payload

**Files:**
- Modify: `src/services/redis_service.rs`
- Test: `src/services/redis_service.rs`

- [ ] **Step 1: Write a failing pure ordering test**

Add a private helper and tests describing the `LPUSH` invariant:

```rust
fn newest_buffer_item(items: &[String]) -> Option<&str> {
    items.first().map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::newest_buffer_item;

    #[test]
    fn test_newest_buffer_item_lpush_order_returns_head() {
        let items = vec!["newest".to_string(), "older".to_string()];
        assert_eq!(newest_buffer_item(&items), Some("newest"));
    }

    #[test]
    fn test_newest_buffer_item_empty_returns_none() {
        assert_eq!(newest_buffer_item(&[]), None);
    }
}
```

Use this helper in the buffer parsing path when tests provide an already-read list; production Redis consumption returns only the same head element atomically. This keeps the ordering rule executable rather than leaving dead test-only logic.

- [ ] **Step 2: Run the test and verify the missing helper fails**

Run: `cargo test --lib redis_service::tests -- --nocapture`

Expected: FAIL until `newest_buffer_item` is defined.

- [ ] **Step 3: Replace tail draining with atomic newest consumption**

Replace `lrange_and_trim` with:

```rust
pub async fn take_latest_and_clear(&self, key: &str) -> Result<Option<String>, AppError> {
    let mut conn = self.conn.clone();
    let script = redis::Script::new(
        r#"
        local latest = redis.call('LINDEX', KEYS[1], 0)
        if latest then
            redis.call('DEL', KEYS[1])
        end
        return latest
        "#,
    );

    script
        .key(key)
        .invoke_async(&mut conn)
        .await
        .map_err(|e| AppError::Internal(format!("Redis take latest error: {e}")))
}
```

Remove the unused batch constant. Keep `LPUSH` and its TTL unchanged.

- [ ] **Step 4: Run focused tests**

Run: `cargo test --lib redis_service::tests -- --nocapture`

Expected: both ordering tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/services/redis_service.rs
git commit -m "fix: consume newest buffered sensor payload"
```

### Task 2: Make Each Scheduler Tick Insert Exactly One Row

**Files:**
- Modify: `src/services/sensor_service.rs`
- Modify: `src/services/heartbeat.rs`
- Test: `src/services/heartbeat.rs`

- [ ] **Step 1: Write failing scheduler decision tests**

Add a pure decision enum and tests:

```rust
#[derive(Debug, PartialEq, Eq)]
enum SensorTickOutcome {
    SensorInserted,
    HeartbeatRequired,
}

fn tick_outcome(flushed_rows: usize) -> SensorTickOutcome {
    if flushed_rows > 0 {
        SensorTickOutcome::SensorInserted
    } else {
        SensorTickOutcome::HeartbeatRequired
    }
}

#[cfg(test)]
mod tests {
    use super::{tick_outcome, SensorTickOutcome};

    #[test]
    fn test_tick_outcome_sensor_insert_suppresses_heartbeat() {
        assert_eq!(tick_outcome(1), SensorTickOutcome::SensorInserted);
    }

    #[test]
    fn test_tick_outcome_empty_buffer_requires_heartbeat() {
        assert_eq!(tick_outcome(0), SensorTickOutcome::HeartbeatRequired);
    }
}
```

- [ ] **Step 2: Run tests and verify current unconditional heartbeat behavior is uncovered**

Run: `cargo test --lib heartbeat::tests -- --nocapture`

Expected: FAIL before the scheduler uses `tick_outcome`.

- [ ] **Step 3: Update sensor flush and timestamps**

Change `flush_sensor_buffer` to use `take_latest_and_clear`. Return `Ok(0)` for an empty or malformed payload and `Ok(1)` only after a successful insert. Change all sensor and heartbeat inserts from `date_trunc('minute', NOW())` to `NOW()` so five-second samples retain their actual order. Remove the unused `RETURNING` row in the direct insert and use `.execute(pool)`.

The buffer extraction must be:

```rust
let latest_json = match redis.take_latest_and_clear(SENSOR_BUFFER_KEY).await? {
    Some(json) => json,
    None => return Ok(0),
};

let payload = match serde_json::from_str::<SensorPayload>(&latest_json) {
    Ok(payload) => payload,
    Err(error) => {
        tracing::warn!("Skipping malformed sensor buffer entry: {error}");
        return Ok(0);
    }
};
```

- [ ] **Step 4: Make heartbeat conditional**

At each tick initialize `flushed_rows` to zero, call the Redis flush when available, and insert a heartbeat only for `SensorTickOutcome::HeartbeatRequired`:

```rust
let flushed_rows = if let Some(ref redis) = redis {
    match sensor_service::flush_sensor_buffer(redis, &pool).await {
        Ok(rows) => rows,
        Err(error) => {
            tracing::warn!("Sensor buffer flush failed: {error}");
            0
        }
    }
} else {
    0
};

if tick_outcome(flushed_rows) == SensorTickOutcome::HeartbeatRequired {
    if let Err(error) = sensor_service::insert_heartbeat(&pool).await {
        tracing::warn!("Heartbeat insert failed: {error}");
    }
}
```

- [ ] **Step 5: Run focused tests and build**

Run: `cargo test --lib heartbeat::tests -- --nocapture`

Expected: scheduler tests PASS.

Run: `cargo test --lib redis_service::tests -- --nocapture`

Expected: Redis unit tests PASS.

Run: `cargo build`

Expected: PASS without the unused `row` warning.

- [ ] **Step 6: Commit**

```bash
git add src/services/sensor_service.rs src/services/heartbeat.rs
git commit -m "fix: write one sensor snapshot per interval"
```

### Task 3: Stabilize `/sensors/latest` for One Minute

**Files:**
- Modify: `src/services/sensor_service.rs`
- Test: `tests/sensor_tests.rs`
- Modify: `tests/common/mod.rs`

- [ ] **Step 1: Update test fixtures to the current unchanged contract**

Use the existing fields in every `SensorPayload` fixture:

```rust
SensorPayload {
    lux_panel_left: Some(202),
    lux_panel_right: None,
    lux_l: None,
    lux_ml: None,
    lux_mr: None,
    lux_r: None,
    voltage: None,
    current: None,
    power: None,
}
```

Assert `result.success` and clean inserted rows by querying their marker values. Change cleanup SQL to:

```rust
sqlx::query("DELETE FROM sensor_logs WHERE id = $1")
```

- [ ] **Step 2: Add failing database tests for transient and expired NULL values**

Insert controlled `sensor_logs` rows directly in each test transaction/setup:

```rust
sqlx::query(
    "INSERT INTO sensor_logs
     (timestamp_slot, lux_panel_left, is_online)
     VALUES (NOW() - INTERVAL '30 seconds', 202, TRUE),
            (NOW(), NULL, FALSE)"
)
.execute(&pool)
.await
.expect("test rows should insert");

let latest = sensor_service::get_latest_reading(&pool)
    .await
    .expect("latest query should succeed")
    .expect("latest row should exist");
assert_eq!(latest.lux_panel_left, Some(202));
assert!(latest.is_online);
```

Add a second case with the value timestamp at `NOW() - INTERVAL '61 seconds'`; expect `lux_panel_left == None` and `is_online == false`.

- [ ] **Step 3: Run the tests and verify current latest-row query fails**

Run: `cargo test --test sensor_tests test_get_latest_reading -- --nocapture --test-threads=1`

Expected: transient-null assertion FAIL against the current query.

- [ ] **Step 4: Implement one-minute per-field fallback SQL**

Replace `get_latest_reading` with one query based on the newest raw row and correlated lookups for every field:

```sql
WITH latest_row AS (
    SELECT id, timestamp_slot
    FROM sensor_logs
    ORDER BY timestamp_slot DESC, id DESC
    LIMIT 1
)
SELECT
    latest_row.id,
    latest_row.timestamp_slot,
    (SELECT lux_l FROM sensor_logs WHERE lux_l IS NOT NULL AND timestamp_slot >= NOW() - INTERVAL '1 minute' ORDER BY timestamp_slot DESC, id DESC LIMIT 1) AS lux_l,
    (SELECT lux_ml FROM sensor_logs WHERE lux_ml IS NOT NULL AND timestamp_slot >= NOW() - INTERVAL '1 minute' ORDER BY timestamp_slot DESC, id DESC LIMIT 1) AS lux_ml,
    (SELECT lux_mr FROM sensor_logs WHERE lux_mr IS NOT NULL AND timestamp_slot >= NOW() - INTERVAL '1 minute' ORDER BY timestamp_slot DESC, id DESC LIMIT 1) AS lux_mr,
    (SELECT lux_r FROM sensor_logs WHERE lux_r IS NOT NULL AND timestamp_slot >= NOW() - INTERVAL '1 minute' ORDER BY timestamp_slot DESC, id DESC LIMIT 1) AS lux_r,
    (SELECT lux_panel_left FROM sensor_logs WHERE lux_panel_left IS NOT NULL AND timestamp_slot >= NOW() - INTERVAL '1 minute' ORDER BY timestamp_slot DESC, id DESC LIMIT 1) AS lux_panel_left,
    (SELECT lux_panel_right FROM sensor_logs WHERE lux_panel_right IS NOT NULL AND timestamp_slot >= NOW() - INTERVAL '1 minute' ORDER BY timestamp_slot DESC, id DESC LIMIT 1) AS lux_panel_right,
    (SELECT voltage FROM sensor_logs WHERE voltage IS NOT NULL AND timestamp_slot >= NOW() - INTERVAL '1 minute' ORDER BY timestamp_slot DESC, id DESC LIMIT 1) AS voltage,
    (SELECT current FROM sensor_logs WHERE current IS NOT NULL AND timestamp_slot >= NOW() - INTERVAL '1 minute' ORDER BY timestamp_slot DESC, id DESC LIMIT 1) AS current,
    (SELECT power FROM sensor_logs WHERE power IS NOT NULL AND timestamp_slot >= NOW() - INTERVAL '1 minute' ORDER BY timestamp_slot DESC, id DESC LIMIT 1) AS power,
    EXISTS (
        SELECT 1 FROM sensor_logs
        WHERE is_online = TRUE AND timestamp_slot >= NOW() - INTERVAL '1 minute'
    ) AS is_online
FROM latest_row
```

Also change history ordering to `ORDER BY timestamp_slot DESC, id DESC`.

- [ ] **Step 5: Run sensor tests serially**

Run: `cargo test --test sensor_tests -- --nocapture --test-threads=1`

Expected: PASS with a configured test PostgreSQL database.

- [ ] **Step 6: Commit**

```bash
git add src/services/sensor_service.rs tests/sensor_tests.rs tests/common/mod.rs
git commit -m "fix: retain dashboard sensor values for one minute"
```

### Task 4: Lock HTTP Contracts and Verify the Repository

**Files:**
- Modify: `tests/integration_test.rs`
- Create: `CHANGELOG.md`

- [ ] **Step 1: Add serialization contract assertions**

For POST `/sensors/reading`, send exactly the existing fields and assert status `201` plus response keys `success` and `message`. For authenticated GET `/sensors/latest`, assert the top-level response has only the `reading` contract and, when non-null, the established `SensorLog` fields. Do not edit route or schema files.

```rust
let payload = serde_json::json!({
    "lux_panel_left": 202,
    "lux_panel_right": null,
    "lux_l": null,
    "lux_ml": null,
    "lux_mr": null,
    "lux_r": null,
    "voltage": null,
    "current": null,
    "power": null
});
```

- [ ] **Step 2: Document the internal fix**

Create `CHANGELOG.md` with:

```markdown
# Changelog

## Unreleased

- Fixed sensor scheduling so a five-second interval writes either the newest ESP32 payload or one `NULL` heartbeat, not both.
- Fixed Redis newest-reading selection.
- Stabilized dashboard sensor values across gaps shorter than one minute.
- Preserved all HTTP GET/POST/PATCH contracts.
```

- [ ] **Step 3: Run formatting and static verification**

Run: `cargo fmt --check`

Expected: PASS.

Run: `cargo build`

Expected: PASS without new warnings.

- [ ] **Step 4: Run the complete test suite**

Run: `cargo test --all-targets -- --test-threads=1`

Expected: PASS when PostgreSQL and optional Redis test services are available. If an external service is unavailable, record the exact environmental failure separately from code failures.

- [ ] **Step 5: Verify no HTTP contract changed**

Run: `git diff 0fe0310 -- src/routes src/schemas`

Expected: no output.

- [ ] **Step 6: Commit**

```bash
git add tests/integration_test.rs CHANGELOG.md
git commit -m "test: lock sensor API contracts"
```
