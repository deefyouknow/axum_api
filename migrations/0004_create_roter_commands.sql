-- roter_commands: store motor rotation commands + ESP32 responses
CREATE TABLE IF NOT EXISTS roter_commands (
    id              BIGSERIAL   PRIMARY KEY,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    source          TEXT        NOT NULL DEFAULT 'manual',  -- 'manual' | 'algorithm'
    target_lux_l    INTEGER,
    target_lux_r    INTEGER,
    status          TEXT        NOT NULL DEFAULT 'pending', -- pending | executing | success | failed
    executed_at     TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ,
    lux_left        INTEGER,
    lux_right       INTEGER,
    response_note   TEXT
);

CREATE INDEX IF NOT EXISTS idx_roter_cmd_status ON roter_commands (status);
CREATE INDEX IF NOT EXISTS idx_roter_cmd_time ON roter_commands (created_at DESC);
