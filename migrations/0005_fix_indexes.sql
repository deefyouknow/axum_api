-- ============================================================
-- Migration 0005 — Fix Indexes + Performance
-- ============================================================

-- 1. Fix login 125s → <1ms: index บน certificate.username
--    (SELECT username, password FROM certificate WHERE username = $1 LIMIT 1)
CREATE INDEX IF NOT EXISTS idx_certificate_username
    ON certificate (username);

-- 2. sensor_readings — time index (ถ้ายังไม่มีจาก migration ก่อนหน้า)
CREATE INDEX IF NOT EXISTS idx_sensor_readings_time
    ON sensor_readings (time DESC);
