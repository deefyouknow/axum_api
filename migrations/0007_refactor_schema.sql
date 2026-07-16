-- Drop the old tables if we want to replace them, or rename them to preserve data.
-- Let's rename them first
ALTER TABLE IF EXISTS sensor_readings RENAME TO sensor_readings_old;
ALTER TABLE IF EXISTS roter_commands RENAME TO roter_commands_old;

-- 1. Create sensor_logs with declarative partitioning
CREATE TABLE sensor_logs (
    id BIGSERIAL,
    timestamp_slot TIMESTAMPTZ NOT NULL,
    lux_l INTEGER,
    lux_ml INTEGER,
    lux_mr INTEGER,
    lux_r INTEGER,
    lux_panel_left INTEGER,
    lux_panel_right INTEGER,
    voltage REAL,
    current REAL,
    power REAL,
    is_online BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (id, timestamp_slot)
) PARTITION BY RANGE (timestamp_slot);

-- Create index on timestamp_slot for faster range queries
CREATE INDEX idx_sensor_logs_timestamp ON sensor_logs (timestamp_slot);

-- Create a plpgsql function to automatically create a partition for a given month
CREATE OR REPLACE FUNCTION create_sensor_logs_partition(for_date DATE) RETURNS void AS $$
DECLARE
    start_date DATE := date_trunc('month', for_date);
    end_date DATE := start_date + INTERVAL '1 month';
    partition_name TEXT := 'sensor_logs_y' || to_char(start_date, 'YYYY') || 'm' || to_char(start_date, 'MM');
BEGIN
    EXECUTE format(
        'CREATE TABLE IF NOT EXISTS %I PARTITION OF sensor_logs FOR VALUES FROM (%L) TO (%L)',
        partition_name, start_date, end_date
    );
END;
$$ LANGUAGE plpgsql;

-- Pre-create partitions for the current month and the next month
DO $$
BEGIN
    PERFORM create_sensor_logs_partition(CURRENT_DATE);
    PERFORM create_sensor_logs_partition(CURRENT_DATE + INTERVAL '1 month');
END $$;


-- 2. Create active_commands table
CREATE TABLE active_commands (
    id BIGSERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ NULL,
    function_name VARCHAR(20) NOT NULL DEFAULT 'active',
    from_user VARCHAR(100) NOT NULL,
    target_type VARCHAR(20) NOT NULL,
    target_value REAL NULL,
    target_left_ratio REAL NULL,
    target_right_ratio REAL NULL,
    tolerance REAL NOT NULL,
    lux_left INTEGER NULL,
    lux_right INTEGER NULL,
    status SMALLINT NOT NULL DEFAULT 0
);

-- Index for querying pending commands
CREATE INDEX idx_active_commands_status ON active_commands (status, created_at);
