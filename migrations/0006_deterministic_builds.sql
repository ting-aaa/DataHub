ALTER TABLE datahub_jobs
    ADD COLUMN input_hash TEXT,
    ADD COLUMN manifest JSONB,
    ADD CONSTRAINT datahub_jobs_input_hash_length
        CHECK (input_hash IS NULL OR LENGTH(input_hash) = 64);

CREATE INDEX datahub_jobs_project_input_hash_idx
    ON datahub_jobs (project_id, input_hash)
    WHERE kind = 'build' AND input_hash IS NOT NULL;
