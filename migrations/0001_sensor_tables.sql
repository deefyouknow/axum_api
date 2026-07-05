-- ============================================================
-- Sensor Tables Migration — Single-Pipe Design
-- timestamp บันทึกฝั่ง server เสมอ (DEFAULT NOW())
-- ไม่ใช้ timestamp จาก ESP32 เพื่อหลีกเลี่ยง clock drift
-- ===========================================================

-- 1. แสงบนแผง Solar Cell (BH1750FVI ซ้าย-ขวา)
CREATE TABLE IF NOT EXISTS solar_lux_readings (
    id          BIGSERIAL   PRIMARY KEY,
    time        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    lux_left    INTEGER,            -- nullable: ถ้าเซนเซอร์ซ้ายไม่มีข้อมูล
    lux_right   INTEGER             -- nullable: ถ้าเซนเซอร์ขวาไม่มีข้อมูล
);
CREATE INDEX IF NOT EXISTS idx_solar_lux_time ON solar_lux_readings (time DESC);

-- 2. Array เซนเซอร์ทิศทาง (BH1750FVI x4 ผ่าน TCA9548A)
CREATE TABLE IF NOT EXISTS array_lux_readings (
    id          BIGSERIAL   PRIMARY KEY,
    time        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    lux_l       INTEGER,            -- ซ้ายสุด
    lux_ml      INTEGER,            -- กลางซ้าย
    lux_mr      INTEGER,            -- กลางขวา
    lux_r       INTEGER             -- ขวาสุด
);
CREATE INDEX IF NOT EXISTS idx_array_lux_time ON array_lux_readings (time DESC);

-- 3. ตำแหน่งและสถานะ Roter (AS5600 + limit switches)
CREATE TABLE IF NOT EXISTS roter_readings (
    id              BIGSERIAL   PRIMARY KEY,
    time            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    roter_angle     INTEGER,                -- องศา 0–360 จาก AS5600
    limit_sw_left   BOOLEAN     DEFAULT TRUE,
    limit_sw_right  BOOLEAN     DEFAULT TRUE
);
CREATE INDEX IF NOT EXISTS idx_roter_time ON roter_readings (time DESC);

-- 4. Log คำสั่งควบคุม (สร้างเองจาก backend ไม่ใช่จาก ESP32)
CREATE TABLE IF NOT EXISTS roter_commands (
    id              BIGSERIAL   PRIMARY KEY,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    target_angle    INTEGER     NOT NULL,
    before_angle    INTEGER     NOT NULL,
    source          TEXT        NOT NULL DEFAULT 'manual'  -- 'manual' | 'algorithm'
);
CREATE INDEX IF NOT EXISTS idx_roter_cmd_time ON roter_commands (created_at DESC);