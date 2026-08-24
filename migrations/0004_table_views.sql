CREATE TABLE datahub_table_views (
    id UUID PRIMARY KEY,
    project_id UUID NOT NULL REFERENCES datahub_projects(id) ON DELETE CASCADE,
    schema_id UUID NOT NULL REFERENCES datahub_schemas(id) ON DELETE CASCADE,
    created_by UUID NOT NULL REFERENCES datahub_users(id) ON DELETE CASCADE,
    block_size INTEGER NOT NULL CHECK (block_size BETWEEN 256 AND 1024),
    sort_spec JSONB NOT NULL DEFAULT '[]'::JSONB,
    filter_spec JSONB NOT NULL DEFAULT '[]'::JSONB,
    data_revision_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '1 hour'
);

CREATE INDEX datahub_table_views_expiry_idx ON datahub_table_views (expires_at);
