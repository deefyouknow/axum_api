-- ============================================================
-- Sensor Readings — Single Table + Partition by Day
-- Heartbeat Model: all sensor fields in one table, nullable
-- timestamp บันทึกฝั่ง server เสมอ (DEFAULT NOW())
-- ============================================================

-- 1. Parent table — partitioned by RANGE on time column
CREATE TABLE sensor_readings (
    id              BIGSERIAL,
    time            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Solar lux group (BH1750FVI left/right)
    lux_left        INTEGER,
    lux_right       INTEGER,
    -- Array lux group (BH1750FVI x4 via TCA9548A)
    lux_l           INTEGER,
    lux_ml          INTEGER,
    lux_mr          INTEGER,
    lux_r           INTEGER,
    -- Roter group (AS5600 + limit switches)
    roter_angle     INTEGER,
    limit_sw_left   BOOLEAN,
    limit_sw_right  BOOLEAN,
    PRIMARY KEY (id, time)
) PARTITION BY RANGE (time);

-- 2. Index on parent — propagates to all child partitions
CREATE INDEX idx_sensor_readings_time ON sensor_readings (time DESC);

-- 3. Daily partitions (ตัวอย่าง 7 วัน: 2026-07-05 ~ 2026-07-11)
CREATE TABLE sensor_readings_2026_07_05 PARTITION OF sensor_readings
    FOR VALUES FROM ('2026-07-05') TO ('2026-07-06');
CREATE TABLE sensor_readings_2026_07_06 PARTITION OF sensor_readings
    FOR VALUES FROM ('2026-07-06') TO ('2026-07-07');
CREATE TABLE sensor_readings_2026_07_07 PARTITION OF sensor_readings
    FOR VALUES FROM ('2026-07-07') TO ('2026-07-08');
CREATE TABLE sensor_readings_2026_07_08 PARTITION OF sensor_readings
    FOR VALUES FROM ('2026-07-08') TO ('2026-07-09');
CREATE TABLE sensor_readings_2026_07_09 PARTITION OF sensor_readings
    FOR VALUES FROM ('2026-07-09') TO ('2026-07-10');
CREATE TABLE sensor_readings_2026_07_10 PARTITION OF sensor_readings
    FOR VALUES FROM ('2026-07-10') TO ('2026-07-11');
CREATE TABLE sensor_readings_2026_07_11 PARTITION OF sensor_readings
    FOR VALUES FROM ('2026-07-11') TO ('2026-07-12');

-- 4. Default partition — catch-all for data outside defined ranges
CREATE TABLE sensor_readings_default PARTITION OF sensor_readings DEFAULT;
