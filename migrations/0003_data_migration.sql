-- ============================================================
-- Data Migration — old tables → sensor_readings
-- Run AFTER 0002_sensor_readings.sql
-- ============================================================

-- 1. Migrate solar_lux_readings → sensor_readings
INSERT INTO sensor_readings (time, lux_left, lux_right)
SELECT time, lux_left, lux_right
FROM solar_lux_readings;

-- 2. Migrate array_lux_readings → sensor_readings
INSERT INTO sensor_readings (time, lux_l, lux_ml, lux_mr, lux_r)
SELECT time, lux_l, lux_ml, lux_mr, lux_r
FROM array_lux_readings;

-- 3. Migrate roter_readings → sensor_readings
INSERT INTO sensor_readings (time, roter_angle, limit_sw_left, limit_sw_right)
SELECT time, roter_angle, limit_sw_left, limit_sw_right
FROM roter_readings;

-- 4. Drop old tables (after verifying migration)
DROP TABLE IF EXISTS solar_lux_readings;
DROP TABLE IF EXISTS array_lux_readings;
DROP TABLE IF EXISTS roter_readings;
