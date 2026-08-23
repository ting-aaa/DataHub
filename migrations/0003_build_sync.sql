CREATE TABLE datahub_data_revisions (
    revision_id UUID PRIMARY KEY,
    project_id UUID NOT NULL REFERENCES datahub_projects(id) ON DELETE CASCADE,
    schema_id UUID NOT NULL REFERENCES datahub_schemas(id) ON DELETE CASCADE,
    row_id UUID NOT NULL REFERENCES datahub_config_rows(id) ON DELETE CASCADE,
    row_revision_id UUID NOT NULL REFERENCES datahub_row_revisions(revision_id) ON DELETE CASCADE,
    actor_id UUID NOT NULL REFERENCES datahub_users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX datahub_data_revisions_project_created_idx
    ON datahub_data_revisions (project_id, created_at DESC);

CREATE TABLE datahub_projection_schemas (
    project_id UUID NOT NULL,
    schema_id UUID NOT NULL,
    document JSONB NOT NULL,
    source_version BIGINT NOT NULL,
    source_event_id UUID NOT NULL UNIQUE,
    synced_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (project_id, schema_id)
);

CREATE TABLE datahub_projection_rows (
    project_id UUID NOT NULL,
    schema_id UUID NOT NULL,
    row_id UUID NOT NULL,
    document JSONB NOT NULL,
    source_version BIGINT NOT NULL,
    source_event_id UUID NOT NULL UNIQUE,
    synced_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (project_id, schema_id, row_id)
);

CREATE TABLE datahub_build_artifacts (
    build_id UUID NOT NULL REFERENCES datahub_jobs(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    media_type TEXT NOT NULL,
    sha256 CHAR(64) NOT NULL,
    content BYTEA NOT NULL,
    PRIMARY KEY (build_id, path)
);

CREATE INDEX datahub_build_artifacts_build_idx ON datahub_build_artifacts (build_id);
