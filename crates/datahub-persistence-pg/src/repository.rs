use datahub_kernel::{
    AuditEventId, BuildId, ConfigRow, OutboxEventId, ProjectId, ProjectRole, RevisionId,
    SchemaDefinition, SchemaId, SessionId, UserId,
};
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Row, Transaction};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("record was not found")]
    NotFound,
    #[error("optimistic version conflict")]
    Conflict,
    #[error("stored document is invalid")]
    InvalidDocument(#[from] serde_json::Error),
    #[error("stored project role is invalid")]
    InvalidRole,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserAccount {
    pub id: UserId,
    pub username: String,
    pub is_system_admin: bool,
    #[serde(skip_serializing)]
    pub password_hash: String,
}

#[derive(Debug, Clone)]
pub struct SessionPrincipal {
    pub user_id: UserId,
    pub username: String,
    pub is_system_admin: bool,
    pub csrf_digest: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectRecord {
    pub id: ProjectId,
    pub name: String,
    pub description: String,
    pub role: ProjectRole,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StoredSchema {
    pub definition: SchemaDefinition,
    pub version: i64,
    pub revision_id: RevisionId,
}

#[derive(Debug, Clone, Serialize)]
pub struct StoredRow {
    pub row: ConfigRow,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildArtifact {
    pub path: String,
    pub media_type: String,
    pub sha256: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildRecord {
    pub id: BuildId,
    pub project_id: ProjectId,
    pub target: String,
    pub status: String,
    pub artifacts: Vec<BuildArtifact>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncStatus {
    pub pending: i64,
    pub processed: i64,
    pub failed: i64,
    pub projected_schemas: i64,
    pub projected_rows: i64,
}

/// Counts local accounts.
///
/// # Errors
/// Returns a database error when the count cannot be read.
pub async fn user_count(pool: &PgPool) -> Result<i64, RepositoryError> {
    Ok(sqlx::query_scalar("SELECT COUNT(*) FROM datahub_users")
        .fetch_one(pool)
        .await?)
}

/// Creates a local account.
///
/// # Errors
/// Returns a database error for invalid or duplicate account data.
pub async fn create_user(
    pool: &PgPool,
    id: UserId,
    username: &str,
    password_hash: &str,
) -> Result<UserAccount, RepositoryError> {
    let row = sqlx::query(
        "INSERT INTO datahub_users (id, username, password_hash) VALUES ($1, $2, $3) RETURNING id, username, password_hash, is_system_admin",
    )
    .bind(id.as_uuid())
    .bind(username.trim())
    .bind(password_hash)
    .fetch_one(pool)
    .await?;
    user_from_row(&row)
}

/// Creates the first local account under an advisory transaction lock.
///
/// # Errors
/// Returns [`RepositoryError::Conflict`] after bootstrap or a database error.
pub async fn create_initial_user(
    pool: &PgPool,
    id: UserId,
    username: &str,
    password_hash: &str,
) -> Result<UserAccount, RepositoryError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('datahub-initial-user'))")
        .execute(&mut *tx)
        .await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM datahub_users")
        .fetch_one(&mut *tx)
        .await?;
    if count != 0 {
        return Err(RepositoryError::Conflict);
    }
    let row = sqlx::query(
        "INSERT INTO datahub_users (id, username, password_hash, is_system_admin) VALUES ($1, $2, $3, TRUE) RETURNING id, username, password_hash, is_system_admin",
    )
    .bind(id.as_uuid())
    .bind(username.trim())
    .bind(password_hash)
    .fetch_one(&mut *tx)
    .await?;
    let user = user_from_row(&row)?;
    tx.commit().await?;
    Ok(user)
}

/// Finds an enabled account by case-insensitive username.
///
/// # Errors
/// Returns a database error when the lookup fails.
pub async fn user_by_username(
    pool: &PgPool,
    username: &str,
) -> Result<Option<UserAccount>, RepositoryError> {
    let row = sqlx::query(
        "SELECT id, username, password_hash, is_system_admin FROM datahub_users WHERE LOWER(username) = LOWER($1) AND NOT disabled",
    )
    .bind(username.trim())
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(user_from_row).transpose()
}

/// Persists a hashed bearer/CSRF session pair.
///
/// # Errors
/// Returns a database error when the session cannot be created.
pub async fn create_session(
    pool: &PgPool,
    id: SessionId,
    user_id: UserId,
    token_digest: &str,
    csrf_digest: &str,
    ttl_seconds: i32,
) -> Result<(), RepositoryError> {
    sqlx::query(
        "INSERT INTO datahub_sessions (id, user_id, token_digest, csrf_digest, expires_at) VALUES ($1, $2, $3, $4, NOW() + make_interval(secs => $5))",
    )
    .bind(id.as_uuid())
    .bind(user_id.as_uuid())
    .bind(token_digest)
    .bind(csrf_digest)
    .bind(ttl_seconds)
    .execute(pool)
    .await?;
    Ok(())
}

/// Resolves and touches a non-expired session by token digest.
///
/// # Errors
/// Returns a database error when session resolution fails.
pub async fn session_principal(
    pool: &PgPool,
    token_digest: &str,
) -> Result<Option<SessionPrincipal>, RepositoryError> {
    let row = sqlx::query(
        "UPDATE datahub_sessions s SET last_seen_at = NOW() FROM datahub_users u WHERE s.user_id = u.id AND s.token_digest = $1 AND s.expires_at > NOW() AND NOT u.disabled RETURNING u.id, u.username, u.is_system_admin, s.csrf_digest",
    )
    .bind(token_digest)
    .fetch_optional(pool)
    .await?;
    row.map(|row| {
        Ok(SessionPrincipal {
            user_id: UserId::from_uuid(row.try_get("id")?),
            username: row.try_get("username")?,
            is_system_admin: row.try_get("is_system_admin")?,
            csrf_digest: row.try_get("csrf_digest")?,
        })
    })
    .transpose()
}

/// Creates a project, its owner membership, audit record, and outbox event atomically.
///
/// # Errors
/// Returns a database error and rolls back the transaction on failure.
pub async fn create_project(
    pool: &PgPool,
    id: ProjectId,
    actor: UserId,
    name: &str,
    description: &str,
) -> Result<ProjectRecord, RepositoryError> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO datahub_projects (id, name, description, created_by) VALUES ($1, $2, $3, $4)",
    )
    .bind(id.as_uuid())
    .bind(name.trim())
    .bind(description)
    .bind(actor.as_uuid())
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO datahub_project_members (project_id, user_id, role) VALUES ($1, $2, 'admin')",
    )
    .bind(id.as_uuid())
    .bind(actor.as_uuid())
    .execute(&mut *tx)
    .await?;
    append_events(
        &mut tx,
        actor,
        Some(id),
        "project.created",
        "project",
        id.as_uuid(),
        &json!({"name": name.trim()}),
        &format!("project:{id}:created"),
    )
    .await?;
    tx.commit().await?;
    Ok(ProjectRecord {
        id,
        name: name.trim().to_owned(),
        description: description.to_owned(),
        role: ProjectRole::Admin,
        version: 1,
    })
}

/// Lists projects visible to a user.
///
/// # Errors
/// Returns a database or stored-role decoding error.
pub async fn list_projects(
    pool: &PgPool,
    user_id: UserId,
) -> Result<Vec<ProjectRecord>, RepositoryError> {
    let rows = sqlx::query(
        "SELECT p.id, p.name, p.description, p.version, m.role FROM datahub_projects p JOIN datahub_project_members m ON m.project_id = p.id WHERE m.user_id = $1 ORDER BY p.name, p.id",
    )
    .bind(user_id.as_uuid())
    .fetch_all(pool)
    .await?;
    rows.iter().map(project_from_row).collect()
}

/// Gets a user's role in a project.
///
/// # Errors
/// Returns a database error when the membership lookup fails.
pub async fn project_role(
    pool: &PgPool,
    project_id: ProjectId,
    user_id: UserId,
) -> Result<Option<ProjectRole>, RepositoryError> {
    let role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM datahub_project_members WHERE project_id = $1 AND user_id = $2",
    )
    .bind(project_id.as_uuid())
    .bind(user_id.as_uuid())
    .fetch_optional(pool)
    .await?;
    Ok(role.as_deref().and_then(ProjectRole::parse))
}

/// Adds or changes a project membership and appends audit/outbox events.
///
/// # Errors
/// Returns a database error and rolls back on failure.
pub async fn add_project_member(
    pool: &PgPool,
    project_id: ProjectId,
    user_id: UserId,
    role: ProjectRole,
    actor: UserId,
) -> Result<(), RepositoryError> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO datahub_project_members (project_id, user_id, role) VALUES ($1, $2, $3) ON CONFLICT (project_id, user_id) DO UPDATE SET role = EXCLUDED.role",
    )
    .bind(project_id.as_uuid())
    .bind(user_id.as_uuid())
    .bind(role.as_str())
    .execute(&mut *tx)
    .await?;
    append_events(
        &mut tx,
        actor,
        Some(project_id),
        "project.member_changed",
        "user",
        user_id.as_uuid(),
        &json!({"role": role}),
        &format!(
            "project:{project_id}:member:{user_id}:role:{}",
            role.as_str()
        ),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Creates or optimistically updates a schema and immutable revision atomically.
///
/// # Errors
/// Returns conflict, serialization, or database errors.
pub async fn save_schema(
    pool: &PgPool,
    definition: &SchemaDefinition,
    actor: UserId,
    expected_version: Option<i64>,
) -> Result<StoredSchema, RepositoryError> {
    let document = serde_json::to_value(definition)?;
    let revision_id = RevisionId::new();
    let mut tx = pool.begin().await?;
    let version = if let Some(expected) = expected_version {
        let version: Option<i64> = sqlx::query_scalar(
            "UPDATE datahub_schemas SET name = $1, document = $2, version = version + 1, current_revision_id = $3, updated_by = $4, updated_at = NOW() WHERE id = $5 AND project_id = $6 AND version = $7 RETURNING version",
        )
        .bind(definition.name.trim())
        .bind(&document)
        .bind(revision_id.as_uuid())
        .bind(actor.as_uuid())
        .bind(definition.id.as_uuid())
        .bind(definition.project_id.as_uuid())
        .bind(expected)
        .fetch_optional(&mut *tx)
        .await?;
        version.ok_or(RepositoryError::Conflict)?
    } else {
        sqlx::query(
            "INSERT INTO datahub_schemas (id, project_id, name, document, version, current_revision_id, created_by, updated_by) VALUES ($1, $2, $3, $4, 1, $5, $6, $6)",
        )
        .bind(definition.id.as_uuid())
        .bind(definition.project_id.as_uuid())
        .bind(definition.name.trim())
        .bind(&document)
        .bind(revision_id.as_uuid())
        .bind(actor.as_uuid())
        .execute(&mut *tx)
        .await?;
        1
    };
    sqlx::query(
        "INSERT INTO datahub_schema_revisions (revision_id, schema_id, version, snapshot, actor_id) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(revision_id.as_uuid())
    .bind(definition.id.as_uuid())
    .bind(version)
    .bind(&document)
    .bind(actor.as_uuid())
    .execute(&mut *tx)
    .await?;
    append_events(
        &mut tx,
        actor,
        Some(definition.project_id),
        "schema.saved",
        "schema",
        definition.id.as_uuid(),
        &json!({"version": version, "revision_id": revision_id}),
        &format!("schema:{}:version:{version}", definition.id),
    )
    .await?;
    tx.commit().await?;
    Ok(StoredSchema {
        definition: definition.clone(),
        version,
        revision_id,
    })
}

/// Lists current schemas for a project.
///
/// # Errors
/// Returns database or stored-document decoding errors.
pub async fn list_schemas(
    pool: &PgPool,
    project_id: ProjectId,
) -> Result<Vec<StoredSchema>, RepositoryError> {
    let rows = sqlx::query(
        "SELECT document, version, current_revision_id FROM datahub_schemas WHERE project_id = $1 ORDER BY name, id",
    )
    .bind(project_id.as_uuid())
    .fetch_all(pool)
    .await?;
    rows.iter().map(schema_from_row).collect()
}

/// Creates or optimistically updates a row and immutable revision atomically.
///
/// # Errors
/// Returns conflict, serialization, or database errors.
pub async fn save_row(
    pool: &PgPool,
    row: &ConfigRow,
    actor: UserId,
    project_id: ProjectId,
    expected_version: Option<i64>,
) -> Result<StoredRow, RepositoryError> {
    let revision_id = RevisionId::new();
    let mut stored_row = row.clone();
    stored_row.revision_id = revision_id;
    let document = serde_json::to_value(&stored_row)?;
    let mut tx = pool.begin().await?;
    let version = if let Some(expected) = expected_version {
        let version: Option<i64> = sqlx::query_scalar(
            "UPDATE datahub_config_rows SET document = $1, version = version + 1, current_revision_id = $2, updated_by = $3, updated_at = NOW() WHERE id = $4 AND schema_id = $5 AND version = $6 RETURNING version",
        )
        .bind(&document)
        .bind(revision_id.as_uuid())
        .bind(actor.as_uuid())
        .bind(row.id.as_uuid())
        .bind(row.schema_id.as_uuid())
        .bind(expected)
        .fetch_optional(&mut *tx)
        .await?;
        version.ok_or(RepositoryError::Conflict)?
    } else {
        sqlx::query(
            "INSERT INTO datahub_config_rows (id, schema_id, document, version, current_revision_id, created_by, updated_by) VALUES ($1, $2, $3, 1, $4, $5, $5)",
        )
        .bind(row.id.as_uuid())
        .bind(row.schema_id.as_uuid())
        .bind(&document)
        .bind(revision_id.as_uuid())
        .bind(actor.as_uuid())
        .execute(&mut *tx)
        .await?;
        1
    };
    sqlx::query(
        "INSERT INTO datahub_row_revisions (revision_id, row_id, version, snapshot, actor_id) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(revision_id.as_uuid())
    .bind(row.id.as_uuid())
    .bind(version)
    .bind(&document)
    .bind(actor.as_uuid())
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO datahub_data_revisions (revision_id, project_id, schema_id, row_id, row_revision_id, actor_id) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(RevisionId::new().as_uuid())
    .bind(project_id.as_uuid())
    .bind(row.schema_id.as_uuid())
    .bind(row.id.as_uuid())
    .bind(revision_id.as_uuid())
    .bind(actor.as_uuid())
    .execute(&mut *tx)
    .await?;
    append_events(
        &mut tx,
        actor,
        Some(project_id),
        "row.saved",
        "config_row",
        row.id.as_uuid(),
        &json!({"schema_id": row.schema_id, "version": version}),
        &format!("row:{}:version:{version}", row.id),
    )
    .await?;
    tx.commit().await?;
    Ok(StoredRow {
        row: stored_row,
        version,
    })
}

/// Lists current rows for a schema.
///
/// # Errors
/// Returns database or stored-document decoding errors.
pub async fn list_rows(
    pool: &PgPool,
    schema_id: SchemaId,
) -> Result<Vec<StoredRow>, RepositoryError> {
    let rows = sqlx::query(
        "SELECT document, version FROM datahub_config_rows WHERE schema_id = $1 ORDER BY id",
    )
    .bind(schema_id.as_uuid())
    .fetch_all(pool)
    .await?;
    rows.iter().map(row_from_row).collect()
}

/// Checks whether a referenced row exists in the declared schema.
///
/// # Errors
/// Returns database errors when the lookup fails.
pub async fn row_exists(
    pool: &PgPool,
    schema_id: SchemaId,
    row_id: datahub_kernel::RowId,
) -> Result<bool, RepositoryError> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM datahub_config_rows WHERE schema_id = $1 AND id = $2)",
    )
    .bind(schema_id.as_uuid())
    .bind(row_id.as_uuid())
    .fetch_one(pool)
    .await
    .map_err(RepositoryError::from)
}

/// Records a successful deterministic build and its immutable artifacts.
///
/// # Errors
/// Returns serialization or database errors and rolls back atomically.
pub async fn record_build(
    pool: &PgPool,
    id: BuildId,
    project_id: ProjectId,
    actor: UserId,
    target: &str,
    artifacts: &[BuildArtifact],
) -> Result<BuildRecord, RepositoryError> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO datahub_jobs (id, project_id, kind, status, payload, result, attempts) VALUES ($1, $2, 'build', 'succeeded', $3, $4, 1)",
    )
    .bind(id.as_uuid())
    .bind(project_id.as_uuid())
    .bind(json!({"target": target}))
    .bind(json!({"artifact_count": artifacts.len()}))
    .execute(&mut *tx)
    .await?;
    for artifact in artifacts {
        sqlx::query(
            "INSERT INTO datahub_build_artifacts (build_id, path, media_type, sha256, content) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id.as_uuid())
        .bind(&artifact.path)
        .bind(&artifact.media_type)
        .bind(&artifact.sha256)
        .bind(&artifact.content)
        .execute(&mut *tx)
        .await?;
    }
    append_events(
        &mut tx,
        actor,
        Some(project_id),
        "build.succeeded",
        "build",
        id.as_uuid(),
        &json!({"target": target, "artifact_count": artifacts.len()}),
        &format!("build:{id}:succeeded"),
    )
    .await?;
    tx.commit().await?;
    Ok(BuildRecord {
        id,
        project_id,
        target: target.to_owned(),
        status: "succeeded".into(),
        artifacts: artifacts.to_vec(),
    })
}

/// Lists builds and immutable artifacts for a project.
///
/// # Errors
/// Returns database errors when build history cannot be read.
pub async fn list_builds(
    pool: &PgPool,
    project_id: ProjectId,
) -> Result<Vec<BuildRecord>, RepositoryError> {
    let rows = sqlx::query(
        "SELECT id, status, payload->>'target' AS target FROM datahub_jobs WHERE project_id = $1 AND kind = 'build' ORDER BY created_at DESC, id DESC",
    )
    .bind(project_id.as_uuid())
    .fetch_all(pool)
    .await?;
    let mut builds = Vec::with_capacity(rows.len());
    for row in rows {
        let id = BuildId::from_uuid(row.try_get("id")?);
        let artifact_rows = sqlx::query(
            "SELECT path, media_type, sha256, content FROM datahub_build_artifacts WHERE build_id = $1 ORDER BY path",
        )
        .bind(id.as_uuid())
        .fetch_all(pool)
        .await?;
        let artifacts = artifact_rows
            .iter()
            .map(|artifact| {
                Ok(BuildArtifact {
                    path: artifact.try_get("path")?,
                    media_type: artifact.try_get("media_type")?,
                    sha256: artifact.try_get("sha256")?,
                    content: artifact.try_get("content")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()?;
        builds.push(BuildRecord {
            id,
            project_id,
            target: row.try_get("target")?,
            status: row.try_get("status")?,
            artifacts,
        });
    }
    Ok(builds)
}

/// Returns transactional outbox and local `PostgreSQL` projection counts.
///
/// # Errors
/// Returns database errors when the status cannot be read.
pub async fn sync_status(
    pool: &PgPool,
    project_id: ProjectId,
) -> Result<SyncStatus, RepositoryError> {
    let row = sqlx::query(
        "SELECT COUNT(*) FILTER (WHERE processed_at IS NULL AND last_error IS NULL) AS pending, COUNT(*) FILTER (WHERE processed_at IS NOT NULL) AS processed, COUNT(*) FILTER (WHERE processed_at IS NULL AND last_error IS NOT NULL) AS failed FROM datahub_outbox_events WHERE project_id = $1",
    )
    .bind(project_id.as_uuid())
    .fetch_one(pool)
    .await?;
    let projected_schemas =
        sqlx::query_scalar("SELECT COUNT(*) FROM datahub_projection_schemas WHERE project_id = $1")
            .bind(project_id.as_uuid())
            .fetch_one(pool)
            .await?;
    let projected_rows =
        sqlx::query_scalar("SELECT COUNT(*) FROM datahub_projection_rows WHERE project_id = $1")
            .bind(project_id.as_uuid())
            .fetch_one(pool)
            .await?;
    Ok(SyncStatus {
        pending: row.try_get("pending")?,
        processed: row.try_get("processed")?,
        failed: row.try_get("failed")?,
        projected_schemas,
        projected_rows,
    })
}

/// Claims and idempotently applies an outbox batch to the local `PostgreSQL` projection.
///
/// # Errors
/// Returns database errors and leaves the batch retryable after rollback.
pub async fn process_outbox_batch(pool: &PgPool, limit: i64) -> Result<u64, RepositoryError> {
    let mut tx = pool.begin().await?;
    let events = sqlx::query(
        "SELECT id, project_id, event_type, aggregate_id FROM datahub_outbox_events WHERE processed_at IS NULL AND available_at <= NOW() ORDER BY created_at, id FOR UPDATE SKIP LOCKED LIMIT $1",
    )
    .bind(limit)
    .fetch_all(&mut *tx)
    .await?;
    for event in &events {
        let event_id: Uuid = event.try_get("id")?;
        let project_id: Option<Uuid> = event.try_get("project_id")?;
        let event_type: String = event.try_get("event_type")?;
        let aggregate_id: Uuid = event.try_get("aggregate_id")?;
        match (event_type.as_str(), project_id) {
            ("schema.saved", Some(project_id)) => {
                sqlx::query(
                    "INSERT INTO datahub_projection_schemas (project_id, schema_id, document, source_version, source_event_id) SELECT $1, id, document, version, $3 FROM datahub_schemas WHERE id = $2 ON CONFLICT (project_id, schema_id) DO UPDATE SET document = EXCLUDED.document, source_version = EXCLUDED.source_version, source_event_id = EXCLUDED.source_event_id, synced_at = NOW() WHERE datahub_projection_schemas.source_version <= EXCLUDED.source_version",
                )
                .bind(project_id)
                .bind(aggregate_id)
                .bind(event_id)
                .execute(&mut *tx)
                .await?;
            }
            ("row.saved", Some(project_id)) => {
                sqlx::query(
                    "INSERT INTO datahub_projection_rows (project_id, schema_id, row_id, document, source_version, source_event_id) SELECT $1, schema_id, id, document, version, $3 FROM datahub_config_rows WHERE id = $2 ON CONFLICT (project_id, schema_id, row_id) DO UPDATE SET document = EXCLUDED.document, source_version = EXCLUDED.source_version, source_event_id = EXCLUDED.source_event_id, synced_at = NOW() WHERE datahub_projection_rows.source_version <= EXCLUDED.source_version",
                )
                .bind(project_id)
                .bind(aggregate_id)
                .bind(event_id)
                .execute(&mut *tx)
                .await?;
            }
            _ => {}
        }
        sqlx::query(
            "UPDATE datahub_outbox_events SET processed_at = NOW(), attempts = attempts + 1, last_error = NULL WHERE id = $1",
        )
        .bind(event_id)
        .execute(&mut *tx)
        .await?;
    }
    let processed = u64::try_from(events.len()).unwrap_or(u64::MAX);
    tx.commit().await?;
    Ok(processed)
}

fn user_from_row(row: &sqlx::postgres::PgRow) -> Result<UserAccount, RepositoryError> {
    Ok(UserAccount {
        id: UserId::from_uuid(row.try_get("id")?),
        username: row.try_get("username")?,
        is_system_admin: row.try_get("is_system_admin")?,
        password_hash: row.try_get("password_hash")?,
    })
}

fn project_from_row(row: &sqlx::postgres::PgRow) -> Result<ProjectRecord, RepositoryError> {
    let role: String = row.try_get("role")?;
    Ok(ProjectRecord {
        id: ProjectId::from_uuid(row.try_get("id")?),
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        role: ProjectRole::parse(&role).ok_or(RepositoryError::InvalidRole)?,
        version: row.try_get("version")?,
    })
}

fn schema_from_row(row: &sqlx::postgres::PgRow) -> Result<StoredSchema, RepositoryError> {
    let document: Value = row.try_get("document")?;
    Ok(StoredSchema {
        definition: serde_json::from_value(document)?,
        version: row.try_get("version")?,
        revision_id: RevisionId::from_uuid(row.try_get("current_revision_id")?),
    })
}

fn row_from_row(row: &sqlx::postgres::PgRow) -> Result<StoredRow, RepositoryError> {
    let document: Value = row.try_get("document")?;
    Ok(StoredRow {
        row: serde_json::from_value(document)?,
        version: row.try_get("version")?,
    })
}

#[allow(clippy::too_many_arguments)]
async fn append_events(
    tx: &mut Transaction<'_, Postgres>,
    actor: UserId,
    project_id: Option<ProjectId>,
    action: &str,
    entity_type: &str,
    entity_id: Uuid,
    details: &Value,
    idempotency_key: &str,
) -> Result<(), RepositoryError> {
    sqlx::query(
        "INSERT INTO datahub_audit_events (id, actor_id, project_id, action, entity_type, entity_id, details) VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(AuditEventId::new().as_uuid())
    .bind(actor.as_uuid())
    .bind(project_id.map(ProjectId::as_uuid))
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(details)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO datahub_outbox_events (id, project_id, event_type, aggregate_type, aggregate_id, payload, idempotency_key) VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (idempotency_key) DO NOTHING",
    )
    .bind(OutboxEventId::new().as_uuid())
    .bind(project_id.map(ProjectId::as_uuid))
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(details)
    .bind(idempotency_key)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
