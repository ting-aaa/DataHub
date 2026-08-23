CREATE TABLE datahub_users (
    id UUID PRIMARY KEY,
    username TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    is_system_admin BOOLEAN NOT NULL DEFAULT FALSE,
    disabled BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT datahub_users_username_length CHECK (char_length(username) BETWEEN 3 AND 64)
);

CREATE UNIQUE INDEX datahub_users_username_lower_uq
    ON datahub_users (LOWER(username));

CREATE TABLE datahub_sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES datahub_users(id) ON DELETE CASCADE,
    token_digest CHAR(64) NOT NULL UNIQUE,
    csrf_digest CHAR(64) NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX datahub_sessions_user_id_idx ON datahub_sessions (user_id);
CREATE INDEX datahub_sessions_expires_at_idx ON datahub_sessions (expires_at);

CREATE TABLE datahub_projects (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    version BIGINT NOT NULL DEFAULT 1,
    created_by UUID NOT NULL REFERENCES datahub_users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT datahub_projects_name_length CHECK (char_length(name) BETWEEN 1 AND 128)
);

CREATE TABLE datahub_project_members (
    project_id UUID NOT NULL REFERENCES datahub_projects(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES datahub_users(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('viewer', 'editor', 'approver', 'admin')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (project_id, user_id)
);

CREATE INDEX datahub_project_members_user_id_idx ON datahub_project_members (user_id);

CREATE TABLE datahub_schemas (
    id UUID PRIMARY KEY,
    project_id UUID NOT NULL REFERENCES datahub_projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    document JSONB NOT NULL,
    version BIGINT NOT NULL,
    current_revision_id UUID NOT NULL,
    created_by UUID NOT NULL REFERENCES datahub_users(id),
    updated_by UUID NOT NULL REFERENCES datahub_users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (project_id, name)
);

CREATE INDEX datahub_schemas_project_id_idx ON datahub_schemas (project_id);

CREATE TABLE datahub_schema_revisions (
    revision_id UUID PRIMARY KEY,
    schema_id UUID NOT NULL REFERENCES datahub_schemas(id) ON DELETE CASCADE,
    version BIGINT NOT NULL,
    snapshot JSONB NOT NULL,
    actor_id UUID NOT NULL REFERENCES datahub_users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (schema_id, version)
);

ALTER TABLE datahub_schemas
    ADD CONSTRAINT datahub_schemas_current_revision_fk
    FOREIGN KEY (current_revision_id) REFERENCES datahub_schema_revisions(revision_id)
    DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE datahub_config_rows (
    id UUID PRIMARY KEY,
    schema_id UUID NOT NULL REFERENCES datahub_schemas(id) ON DELETE CASCADE,
    document JSONB NOT NULL,
    version BIGINT NOT NULL,
    current_revision_id UUID NOT NULL,
    created_by UUID NOT NULL REFERENCES datahub_users(id),
    updated_by UUID NOT NULL REFERENCES datahub_users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX datahub_config_rows_schema_id_idx ON datahub_config_rows (schema_id);

CREATE TABLE datahub_row_revisions (
    revision_id UUID PRIMARY KEY,
    row_id UUID NOT NULL REFERENCES datahub_config_rows(id) ON DELETE CASCADE,
    version BIGINT NOT NULL,
    snapshot JSONB NOT NULL,
    actor_id UUID NOT NULL REFERENCES datahub_users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (row_id, version)
);

ALTER TABLE datahub_config_rows
    ADD CONSTRAINT datahub_config_rows_current_revision_fk
    FOREIGN KEY (current_revision_id) REFERENCES datahub_row_revisions(revision_id)
    DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE datahub_change_sets (
    id UUID PRIMARY KEY,
    project_id UUID NOT NULL REFERENCES datahub_projects(id) ON DELETE CASCADE,
    base_version BIGINT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('draft', 'submitted', 'approved', 'rejected', 'applied')),
    changes JSONB NOT NULL,
    created_by UUID NOT NULL REFERENCES datahub_users(id),
    approved_by UUID REFERENCES datahub_users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE datahub_jobs (
    id UUID PRIMARY KEY,
    project_id UUID REFERENCES datahub_projects(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'dead_letter')),
    payload JSONB NOT NULL,
    result JSONB,
    error TEXT,
    attempts INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX datahub_jobs_status_created_idx ON datahub_jobs (status, created_at);

CREATE TABLE datahub_audit_events (
    id UUID PRIMARY KEY,
    actor_id UUID REFERENCES datahub_users(id),
    project_id UUID REFERENCES datahub_projects(id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id UUID,
    details JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX datahub_audit_events_project_created_idx
    ON datahub_audit_events (project_id, created_at DESC);

CREATE TABLE datahub_outbox_events (
    id UUID PRIMARY KEY,
    project_id UUID REFERENCES datahub_projects(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    aggregate_type TEXT NOT NULL,
    aggregate_id UUID NOT NULL,
    payload JSONB NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    attempts INTEGER NOT NULL DEFAULT 0,
    available_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX datahub_outbox_events_pending_idx
    ON datahub_outbox_events (available_at, created_at)
    WHERE processed_at IS NULL;
