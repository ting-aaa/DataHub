ALTER TABLE datahub_audit_events
    ADD COLUMN correlation_id UUID;

CREATE INDEX datahub_audit_events_project_cursor_idx
    ON datahub_audit_events (project_id, created_at DESC, id DESC);
CREATE INDEX datahub_audit_events_project_correlation_idx
    ON datahub_audit_events (project_id, correlation_id)
    WHERE correlation_id IS NOT NULL;

CREATE TABLE datahub_rate_limit_buckets (
    scope TEXT NOT NULL,
    key_hash CHAR(64) NOT NULL,
    window_started_at TIMESTAMPTZ NOT NULL,
    request_count BIGINT NOT NULL CHECK (request_count > 0),
    PRIMARY KEY (scope, key_hash, window_started_at)
);

CREATE INDEX datahub_rate_limit_buckets_window_idx
    ON datahub_rate_limit_buckets (window_started_at);
