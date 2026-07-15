-- ============================================================
-- Migration 0006 — Replace roter_angle with INA219 power monitor
-- INA219 sensor provides voltage (mV), current (mA), power (mW)
-- All fields nullable: ESP32 may not send every cycle
-- ============================================================

-- 1. Add INA219 columns (nullable, propagate to all partitions)
ALTER TABLE sensor_readings ADD COLUMN ina_voltage INTEGER;
ALTER TABLE sensor_readings ADD COLUMN ina_current INTEGER;
ALTER TABLE sensor_readings ADD COLUMN ina_power  INTEGER;

-- 2. Drop obsolete rotary angle column
ALTER TABLE sensor_readings DROP COLUMN roter_angle;
