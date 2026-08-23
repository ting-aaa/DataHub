CREATE TABLE datahub_formula_sets (
    schema_id UUID PRIMARY KEY REFERENCES datahub_schemas(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES datahub_projects(id) ON DELETE CASCADE,
    schema_revision_id UUID NOT NULL REFERENCES datahub_schema_revisions(revision_id),
    document JSONB NOT NULL,
    version BIGINT NOT NULL,
    current_revision_id UUID NOT NULL,
    created_by UUID NOT NULL REFERENCES datahub_users(id),
    updated_by UUID NOT NULL REFERENCES datahub_users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE datahub_formula_revisions (
    revision_id UUID PRIMARY KEY,
    schema_id UUID NOT NULL REFERENCES datahub_formula_sets(schema_id) ON DELETE CASCADE,
    version BIGINT NOT NULL,
    snapshot JSONB NOT NULL,
    actor_id UUID NOT NULL REFERENCES datahub_users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (schema_id, version)
);

ALTER TABLE datahub_formula_sets
    ADD CONSTRAINT datahub_formula_sets_current_revision_fk
    FOREIGN KEY (current_revision_id) REFERENCES datahub_formula_revisions(revision_id)
    DEFERRABLE INITIALLY DEFERRED;

CREATE INDEX datahub_formula_sets_project_idx ON datahub_formula_sets(project_id);
