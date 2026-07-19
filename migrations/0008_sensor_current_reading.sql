-- ============================================================
-- sensor_current_reading — single-row table for latest sensor data
-- id = 0 ตลอด (fixed ID) → UPSERT อัพเดทค่าเดิม ไม่สร้าง row ใหม่
-- ไม่ partitioned เพราะมีแค่ row เดียว
-- ============================================================

CREATE TABLE IF NOT EXISTS sensor_current_reading (
    id              INTEGER     PRIMARY KEY DEFAULT 0 CHECK (id = 0),
    timestamp_slot  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    lux_l           INTEGER,
    lux_ml          INTEGER,
    lux_mr          INTEGER,
    lux_r           INTEGER,
    lux_panel_left  INTEGER,
    lux_panel_right INTEGER,
    voltage         REAL,
    current         REAL,
    power           REAL,
    is_online       BOOLEAN     NOT NULL DEFAULT TRUE
);

-- Index สำหรับ dashboard query
CREATE INDEX IF NOT EXISTS idx_sensor_current_timestamp
    ON sensor_current_reading (timestamp_slot DESC);
