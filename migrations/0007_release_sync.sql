ALTER TABLE datahub_outbox_events
    ADD COLUMN dead_lettered_at TIMESTAMPTZ;

DROP INDEX datahub_outbox_events_pending_idx;
CREATE INDEX datahub_outbox_events_pending_idx
    ON datahub_outbox_events (available_at, created_at)
    WHERE processed_at IS NULL AND dead_lettered_at IS NULL;

CREATE TABLE datahub_sync_checkpoints (
    project_id UUID PRIMARY KEY REFERENCES datahub_projects(id) ON DELETE CASCADE,
    last_event_id UUID,
    last_processed_at TIMESTAMPTZ,
    status TEXT NOT NULL DEFAULT 'ready' CHECK (status IN ('ready', 'rebuilding', 'failed')),
    last_error TEXT,
    version BIGINT NOT NULL DEFAULT 1
);

CREATE TABLE datahub_projection_plans (
    id UUID PRIMARY KEY,
    project_id UUID NOT NULL REFERENCES datahub_projects(id) ON DELETE CASCADE,
    schema_id UUID NOT NULL REFERENCES datahub_schemas(id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('draft', 'approved', 'applied', 'rejected')),
    destructive BOOLEAN NOT NULL,
    operations JSONB NOT NULL,
    schema_document JSONB NOT NULL,
    created_by UUID NOT NULL REFERENCES datahub_users(id),
    approved_by UUID REFERENCES datahub_users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    approved_at TIMESTAMPTZ,
    applied_at TIMESTAMPTZ
);

CREATE INDEX datahub_projection_plans_project_created_idx
    ON datahub_projection_plans (project_id, created_at DESC);

CREATE TABLE datahub_projection_schema_versions (
    project_id UUID NOT NULL REFERENCES datahub_projects(id) ON DELETE CASCADE,
    schema_id UUID NOT NULL REFERENCES datahub_schemas(id) ON DELETE CASCADE,
    schema_document JSONB NOT NULL,
    plan_id UUID NOT NULL REFERENCES datahub_projection_plans(id),
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (project_id, schema_id)
);

CREATE TABLE datahub_environments (
    id UUID PRIMARY KEY,
    project_id UUID NOT NULL REFERENCES datahub_projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    requires_approval BOOLEAN NOT NULL DEFAULT TRUE,
    current_release_id UUID,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (project_id, name)
);

CREATE TABLE datahub_releases (
    id UUID PRIMARY KEY,
    project_id UUID NOT NULL REFERENCES datahub_projects(id) ON DELETE CASCADE,
    environment_id UUID NOT NULL REFERENCES datahub_environments(id) ON DELETE CASCADE,
    build_id UUID NOT NULL REFERENCES datahub_jobs(id),
    version TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('draft', 'approved', 'published', 'superseded')),
    input_hash CHAR(64) NOT NULL,
    manifest JSONB NOT NULL,
    created_by UUID NOT NULL REFERENCES datahub_users(id),
    approved_by UUID REFERENCES datahub_users(id),
    rollback_of UUID REFERENCES datahub_releases(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    approved_at TIMESTAMPTZ,
    published_at TIMESTAMPTZ,
    UNIQUE (environment_id, version)
);

ALTER TABLE datahub_environments
    ADD CONSTRAINT datahub_environments_current_release_fk
    FOREIGN KEY (current_release_id) REFERENCES datahub_releases(id);

CREATE INDEX datahub_releases_project_created_idx
    ON datahub_releases (project_id, created_at DESC);
