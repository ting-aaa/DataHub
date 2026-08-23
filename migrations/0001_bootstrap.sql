CREATE TABLE IF NOT EXISTS datahub_system_info (
    key TEXT PRIMARY KEY,
    value JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO datahub_system_info (key, value)
VALUES ('schema', '{"version": 1}'::JSONB)
ON CONFLICT (key) DO UPDATE
SET value = EXCLUDED.value,
    updated_at = NOW();
