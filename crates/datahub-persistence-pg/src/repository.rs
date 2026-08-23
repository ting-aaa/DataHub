use datahub_kernel::{
    AuditEventId, BuildId, ConfigRow, EnvironmentId, OutboxEventId, ProjectId, ProjectRole,
    ProjectionPlanId, ReleaseId, RevisionId, SchemaDefinition, SchemaId, SessionId, UserId,
};
use serde::{Deserialize, Serialize};
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
pub struct StoredFormulaSet {
    pub schema_id: SchemaId,
    pub schema_revision_id: RevisionId,
    pub document: Value,
    pub version: i64,
    pub revision_id: RevisionId,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildSchemaSnapshot {
    pub schema: StoredSchema,
    pub rows: Vec<StoredRow>,
    pub data_revision_id: Option<RevisionId>,
}

#[derive(Debug, Clone)]
pub struct RowWrite {
    pub row: ConfigRow,
    pub expected_version: Option<i64>,
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
    pub input_hash: Option<String>,
    pub manifest: Option<Value>,
    pub artifacts: Vec<BuildArtifact>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncStatus {
    pub pending: i64,
    pub retrying: i64,
    pub dead_lettered: i64,
    pub processed: i64,
    pub projected_schemas: i64,
    pub projected_rows: i64,
    pub checkpoint: Option<SyncCheckpoint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncCheckpoint {
    pub last_event_id: Option<Uuid>,
    pub last_processed_at: Option<String>,
    pub status: String,
    pub last_error: Option<String>,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionOperation {
    pub sql: String,
    pub destructive: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectionPlan {
    pub id: ProjectionPlanId,
    pub project_id: ProjectId,
    pub schema_id: SchemaId,
    pub status: String,
    pub destructive: bool,
    pub operations: Vec<ProjectionOperation>,
    pub approved_by: Option<UserId>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnvironmentRecord {
    pub id: EnvironmentId,
    pub project_id: ProjectId,
    pub name: String,
    pub requires_approval: bool,
    pub current_release_id: Option<ReleaseId>,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReleaseRecord {
    pub id: ReleaseId,
    pub project_id: ProjectId,
    pub environment_id: EnvironmentId,
    pub build_id: BuildId,
    pub version: String,
    pub status: String,
    pub input_hash: String,
    pub manifest: Value,
    pub approved_by: Option<UserId>,
    pub rollback_of: Option<ReleaseId>,
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
    let mut saved = save_rows_atomic(
        pool,
        &[RowWrite {
            row: row.clone(),
            expected_version,
        }],
        actor,
        project_id,
        "row.saved",
    )
    .await?;
    saved.pop().ok_or(RepositoryError::NotFound)
}

/// Saves a group of rows, revisions, audit records, and outbox records atomically.
///
/// # Errors
/// Returns conflict, serialization, or database errors. No row is committed if
/// any optimistic version check fails.
pub async fn save_rows_atomic(
    pool: &PgPool,
    writes: &[RowWrite],
    actor: UserId,
    project_id: ProjectId,
    action: &str,
) -> Result<Vec<StoredRow>, RepositoryError> {
    let mut tx = pool.begin().await?;
    let mut saved = Vec::with_capacity(writes.len());
    for write in writes {
        saved.push(save_row_in_transaction(&mut tx, write, actor, project_id, action).await?);
    }
    tx.commit().await?;
    Ok(saved)
}

async fn save_row_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    write: &RowWrite,
    actor: UserId,
    project_id: ProjectId,
    action: &str,
) -> Result<StoredRow, RepositoryError> {
    let revision_id = RevisionId::new();
    let mut stored_row = write.row.clone();
    stored_row.revision_id = revision_id;
    let document = serde_json::to_value(&stored_row)?;
    let version = if let Some(expected) = write.expected_version {
        let version: Option<i64> = sqlx::query_scalar(
            "UPDATE datahub_config_rows SET document = $1, version = version + 1, current_revision_id = $2, updated_by = $3, updated_at = NOW() WHERE id = $4 AND schema_id = $5 AND version = $6 RETURNING version",
        )
        .bind(&document)
        .bind(revision_id.as_uuid())
        .bind(actor.as_uuid())
        .bind(write.row.id.as_uuid())
        .bind(write.row.schema_id.as_uuid())
        .bind(expected)
        .fetch_optional(&mut **tx)
        .await?;
        version.ok_or(RepositoryError::Conflict)?
    } else {
        sqlx::query(
            "INSERT INTO datahub_config_rows (id, schema_id, document, version, current_revision_id, created_by, updated_by) VALUES ($1, $2, $3, 1, $4, $5, $5)",
        )
        .bind(write.row.id.as_uuid())
        .bind(write.row.schema_id.as_uuid())
        .bind(&document)
        .bind(revision_id.as_uuid())
        .bind(actor.as_uuid())
        .execute(&mut **tx)
        .await?;
        1
    };
    sqlx::query(
        "INSERT INTO datahub_row_revisions (revision_id, row_id, version, snapshot, actor_id) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(revision_id.as_uuid())
    .bind(write.row.id.as_uuid())
    .bind(version)
    .bind(&document)
    .bind(actor.as_uuid())
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO datahub_data_revisions (revision_id, project_id, schema_id, row_id, row_revision_id, actor_id) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(RevisionId::new().as_uuid())
    .bind(project_id.as_uuid())
    .bind(write.row.schema_id.as_uuid())
    .bind(write.row.id.as_uuid())
    .bind(revision_id.as_uuid())
    .bind(actor.as_uuid())
    .execute(&mut **tx)
    .await?;
    append_events(
        tx,
        actor,
        Some(project_id),
        action,
        "config_row",
        write.row.id.as_uuid(),
        &json!({"schema_id": write.row.schema_id, "version": version}),
        &format!("{action}:{}:version:{version}", write.row.id),
    )
    .await?;
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

/// Loads all build inputs from one repeatable-read, read-only snapshot.
///
/// # Errors
/// Returns database or stored-document decoding errors.
pub async fn load_build_snapshot(
    pool: &PgPool,
    project_id: ProjectId,
) -> Result<Vec<BuildSchemaSnapshot>, RepositoryError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await?;
    let schema_rows = sqlx::query(
        "SELECT document, version, current_revision_id FROM datahub_schemas WHERE project_id = $1 ORDER BY id",
    )
    .bind(project_id.as_uuid())
    .fetch_all(&mut *tx)
    .await?;
    let mut snapshots = Vec::with_capacity(schema_rows.len());
    for schema_row in &schema_rows {
        let schema = schema_from_row(schema_row)?;
        let row_rows = sqlx::query(
            "SELECT document, version FROM datahub_config_rows WHERE schema_id = $1 ORDER BY id",
        )
        .bind(schema.definition.id.as_uuid())
        .fetch_all(&mut *tx)
        .await?;
        let rows = row_rows
            .iter()
            .map(row_from_row)
            .collect::<Result<Vec<_>, RepositoryError>>()?;
        let data_revision_id = sqlx::query_scalar::<_, Uuid>(
            "SELECT revision_id FROM datahub_data_revisions WHERE schema_id = $1 ORDER BY created_at DESC, revision_id DESC LIMIT 1",
        )
        .bind(schema.definition.id.as_uuid())
        .fetch_optional(&mut *tx)
        .await?
        .map(RevisionId::from_uuid);
        snapshots.push(BuildSchemaSnapshot {
            schema,
            rows,
            data_revision_id,
        });
    }
    tx.commit().await?;
    Ok(snapshots)
}

/// Loads the current formula set for a schema.
///
/// # Errors
/// Returns database errors when the lookup fails.
pub async fn load_formula_set(
    pool: &PgPool,
    schema_id: SchemaId,
) -> Result<Option<StoredFormulaSet>, RepositoryError> {
    let row = sqlx::query(
        "SELECT schema_id, schema_revision_id, document, version, current_revision_id FROM datahub_formula_sets WHERE schema_id = $1",
    )
    .bind(schema_id.as_uuid())
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(formula_set_from_row).transpose()
}

/// Creates or optimistically updates a formula set and its immutable revision.
///
/// # Errors
/// Returns conflict or database errors.
#[allow(clippy::too_many_arguments)]
pub async fn save_formula_set(
    pool: &PgPool,
    project_id: ProjectId,
    schema_id: SchemaId,
    schema_revision_id: RevisionId,
    document: &Value,
    actor: UserId,
    expected_version: Option<i64>,
) -> Result<StoredFormulaSet, RepositoryError> {
    let revision_id = RevisionId::new();
    let mut tx = pool.begin().await?;
    let version = if let Some(expected) = expected_version {
        let version: Option<i64> = sqlx::query_scalar(
            "UPDATE datahub_formula_sets SET schema_revision_id = $1, document = $2, version = version + 1, current_revision_id = $3, updated_by = $4, updated_at = NOW() WHERE schema_id = $5 AND project_id = $6 AND version = $7 RETURNING version",
        )
        .bind(schema_revision_id.as_uuid())
        .bind(document)
        .bind(revision_id.as_uuid())
        .bind(actor.as_uuid())
        .bind(schema_id.as_uuid())
        .bind(project_id.as_uuid())
        .bind(expected)
        .fetch_optional(&mut *tx)
        .await?;
        version.ok_or(RepositoryError::Conflict)?
    } else {
        sqlx::query(
            "INSERT INTO datahub_formula_sets (schema_id, project_id, schema_revision_id, document, version, current_revision_id, created_by, updated_by) VALUES ($1, $2, $3, $4, 1, $5, $6, $6)",
        )
        .bind(schema_id.as_uuid())
        .bind(project_id.as_uuid())
        .bind(schema_revision_id.as_uuid())
        .bind(document)
        .bind(revision_id.as_uuid())
        .bind(actor.as_uuid())
        .execute(&mut *tx)
        .await?;
        1
    };
    sqlx::query(
        "INSERT INTO datahub_formula_revisions (revision_id, schema_id, version, snapshot, actor_id) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(revision_id.as_uuid())
    .bind(schema_id.as_uuid())
    .bind(version)
    .bind(document)
    .bind(actor.as_uuid())
    .execute(&mut *tx)
    .await?;
    append_events(
        &mut tx,
        actor,
        Some(project_id),
        "formula_set.saved",
        "formula_set",
        schema_id.as_uuid(),
        &json!({"schema_revision_id": schema_revision_id, "version": version}),
        &format!("formula-set:{schema_id}:version:{version}"),
    )
    .await?;
    tx.commit().await?;
    Ok(StoredFormulaSet {
        schema_id,
        schema_revision_id,
        document: document.clone(),
        version,
        revision_id,
    })
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
#[allow(clippy::too_many_arguments)]
pub async fn record_build(
    pool: &PgPool,
    id: BuildId,
    project_id: ProjectId,
    actor: UserId,
    target: &str,
    input_hash: &str,
    manifest: &Value,
    artifacts: &[BuildArtifact],
) -> Result<BuildRecord, RepositoryError> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO datahub_jobs (id, project_id, kind, status, payload, result, attempts, input_hash, manifest) VALUES ($1, $2, 'build', 'succeeded', $3, $4, 1, $5, $6)",
    )
    .bind(id.as_uuid())
    .bind(project_id.as_uuid())
    .bind(json!({"target": target}))
    .bind(json!({"artifact_count": artifacts.len()}))
    .bind(input_hash)
    .bind(manifest)
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
        &json!({"target": target, "artifact_count": artifacts.len(), "input_hash": input_hash}),
        &format!("build:{id}:succeeded"),
    )
    .await?;
    tx.commit().await?;
    Ok(BuildRecord {
        id,
        project_id,
        target: target.to_owned(),
        status: "succeeded".into(),
        input_hash: Some(input_hash.to_owned()),
        manifest: Some(manifest.clone()),
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
        "SELECT id, status, payload->>'target' AS target, input_hash, manifest FROM datahub_jobs WHERE project_id = $1 AND kind = 'build' ORDER BY created_at DESC, id DESC",
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
            input_hash: row.try_get("input_hash")?,
            manifest: row.try_get("manifest")?,
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
        "SELECT COUNT(*) FILTER (WHERE processed_at IS NULL AND last_error IS NULL AND dead_lettered_at IS NULL) AS pending, COUNT(*) FILTER (WHERE processed_at IS NULL AND last_error IS NOT NULL AND dead_lettered_at IS NULL) AS retrying, COUNT(*) FILTER (WHERE dead_lettered_at IS NOT NULL) AS dead_lettered, COUNT(*) FILTER (WHERE processed_at IS NOT NULL) AS processed FROM datahub_outbox_events WHERE project_id = $1",
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
    let checkpoint = sqlx::query(
        "SELECT last_event_id, last_processed_at::text, status, last_error, version FROM datahub_sync_checkpoints WHERE project_id = $1",
    )
    .bind(project_id.as_uuid())
    .fetch_optional(pool)
    .await?
    .map(|checkpoint| {
        Ok::<SyncCheckpoint, sqlx::Error>(SyncCheckpoint {
            last_event_id: checkpoint.try_get("last_event_id")?,
            last_processed_at: checkpoint.try_get("last_processed_at")?,
            status: checkpoint.try_get("status")?,
            last_error: checkpoint.try_get("last_error")?,
            version: checkpoint.try_get("version")?,
        })
    })
    .transpose()?;
    Ok(SyncStatus {
        pending: row.try_get("pending")?,
        retrying: row.try_get("retrying")?,
        dead_lettered: row.try_get("dead_lettered")?,
        processed: row.try_get("processed")?,
        projected_schemas,
        projected_rows,
        checkpoint,
    })
}

/// Claims and idempotently applies an outbox batch to the local `PostgreSQL` projection.
///
/// # Errors
/// Returns database errors and leaves the batch retryable after rollback.
pub async fn process_outbox_batch(pool: &PgPool, limit: i64) -> Result<u64, RepositoryError> {
    let mut processed = 0_u64;
    for _ in 0..limit {
        let mut tx = pool.begin().await?;
        let Some(event) = sqlx::query(
            "SELECT id, project_id, event_type, aggregate_id FROM datahub_outbox_events WHERE processed_at IS NULL AND dead_lettered_at IS NULL AND available_at <= NOW() ORDER BY created_at, id FOR UPDATE SKIP LOCKED LIMIT 1",
        )
        .fetch_optional(&mut *tx)
        .await? else {
            tx.rollback().await?;
            break;
        };
        let event_id: Uuid = event.try_get("id")?;
        let project_id: Option<Uuid> = event.try_get("project_id")?;
        let event_type: String = event.try_get("event_type")?;
        let aggregate_id: Uuid = event.try_get("aggregate_id")?;
        let projection = match (event_type.as_str(), project_id) {
            ("schema.saved", Some(project_id)) => {
                let result = sqlx::query(
                    "INSERT INTO datahub_projection_schemas (project_id, schema_id, document, source_version, source_event_id) SELECT $1, id, document, version, $3 FROM datahub_schemas WHERE id = $2 ON CONFLICT (project_id, schema_id) DO UPDATE SET document = EXCLUDED.document, source_version = EXCLUDED.source_version, source_event_id = EXCLUDED.source_event_id, synced_at = NOW() WHERE datahub_projection_schemas.source_version <= EXCLUDED.source_version",
                )
                .bind(project_id)
                .bind(aggregate_id)
                .bind(event_id)
                .execute(&mut *tx)
                .await;
                result.and_then(|done| {
                    if done.rows_affected() == 0 {
                        Err(sqlx::Error::RowNotFound)
                    } else {
                        Ok(())
                    }
                })
            }
            ("row.saved" | "formula.applied" | "xlsx.imported", Some(project_id)) => {
                let result = sqlx::query(
                    "INSERT INTO datahub_projection_rows (project_id, schema_id, row_id, document, source_version, source_event_id) SELECT $1, schema_id, id, document, version, $3 FROM datahub_config_rows WHERE id = $2 ON CONFLICT (project_id, schema_id, row_id) DO UPDATE SET document = EXCLUDED.document, source_version = EXCLUDED.source_version, source_event_id = EXCLUDED.source_event_id, synced_at = NOW() WHERE datahub_projection_rows.source_version <= EXCLUDED.source_version",
                )
                .bind(project_id)
                .bind(aggregate_id)
                .bind(event_id)
                .execute(&mut *tx)
                .await;
                result.and_then(|done| {
                    if done.rows_affected() == 0 {
                        Err(sqlx::Error::RowNotFound)
                    } else {
                        Ok(())
                    }
                })
            }
            _ => Ok(()),
        };
        match projection {
            Ok(()) => {
                sqlx::query(
                    "UPDATE datahub_outbox_events SET processed_at = NOW(), attempts = attempts + 1, last_error = NULL WHERE id = $1",
                )
                .bind(event_id)
                .execute(&mut *tx)
                .await?;
                if let Some(project_id) = project_id {
                    sqlx::query(
                        "INSERT INTO datahub_sync_checkpoints (project_id, last_event_id, last_processed_at) VALUES ($1, $2, NOW()) ON CONFLICT (project_id) DO UPDATE SET last_event_id = EXCLUDED.last_event_id, last_processed_at = EXCLUDED.last_processed_at, status = 'ready', last_error = NULL, version = datahub_sync_checkpoints.version + 1",
                    )
                    .bind(project_id)
                    .bind(event_id)
                    .execute(&mut *tx)
                    .await?;
                }
                tx.commit().await?;
                processed += 1;
            }
            Err(error) => {
                tx.rollback().await?;
                let message = error.to_string();
                sqlx::query(
                    "UPDATE datahub_outbox_events SET attempts = attempts + 1, last_error = $2, available_at = NOW() + make_interval(secs => LEAST(60, CAST(power(2, attempts) AS INTEGER))), dead_lettered_at = CASE WHEN attempts + 1 >= 5 THEN NOW() ELSE NULL END WHERE id = $1 AND processed_at IS NULL",
                )
                .bind(event_id)
                .bind(&message)
                .execute(pool)
                .await?;
                if let Some(project_id) = project_id {
                    sqlx::query(
                        "INSERT INTO datahub_sync_checkpoints (project_id, status, last_error) VALUES ($1, 'failed', $2) ON CONFLICT (project_id) DO UPDATE SET status = 'failed', last_error = EXCLUDED.last_error, version = datahub_sync_checkpoints.version + 1",
                    )
                    .bind(project_id)
                    .bind(message)
                    .execute(pool)
                    .await?;
                }
            }
        }
    }
    Ok(processed)
}

/// Rebuilds all generic projections from canonical project data.
///
/// # Errors
/// Returns database errors and rolls back the rebuild atomically.
pub async fn full_resync(
    pool: &PgPool,
    project_id: ProjectId,
) -> Result<SyncStatus, RepositoryError> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO datahub_sync_checkpoints (project_id, status) VALUES ($1, 'rebuilding') ON CONFLICT (project_id) DO UPDATE SET status = 'rebuilding', last_error = NULL, version = datahub_sync_checkpoints.version + 1",
    )
    .bind(project_id.as_uuid())
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM datahub_projection_rows WHERE project_id = $1")
        .bind(project_id.as_uuid())
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM datahub_projection_schemas WHERE project_id = $1")
        .bind(project_id.as_uuid())
        .execute(&mut *tx)
        .await?;
    let schemas =
        sqlx::query("SELECT id, document, version FROM datahub_schemas WHERE project_id = $1")
            .bind(project_id.as_uuid())
            .fetch_all(&mut *tx)
            .await?;
    for schema in schemas {
        sqlx::query("INSERT INTO datahub_projection_schemas (project_id, schema_id, document, source_version, source_event_id) VALUES ($1, $2, $3, $4, $5)")
            .bind(project_id.as_uuid())
            .bind(schema.try_get::<Uuid, _>("id")?)
            .bind(schema.try_get::<Value, _>("document")?)
            .bind(schema.try_get::<i64, _>("version")?)
            .bind(Uuid::now_v7())
            .execute(&mut *tx)
            .await?;
    }
    let rows = sqlx::query(
        "SELECT r.id, r.schema_id, r.document, r.version FROM datahub_config_rows r JOIN datahub_schemas s ON s.id = r.schema_id WHERE s.project_id = $1",
    )
    .bind(project_id.as_uuid())
    .fetch_all(&mut *tx)
    .await?;
    for row in rows {
        sqlx::query("INSERT INTO datahub_projection_rows (project_id, schema_id, row_id, document, source_version, source_event_id) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(project_id.as_uuid())
            .bind(row.try_get::<Uuid, _>("schema_id")?)
            .bind(row.try_get::<Uuid, _>("id")?)
            .bind(row.try_get::<Value, _>("document")?)
            .bind(row.try_get::<i64, _>("version")?)
            .bind(Uuid::now_v7())
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("UPDATE datahub_sync_checkpoints SET status = 'ready', last_error = NULL, last_processed_at = NOW(), version = version + 1 WHERE project_id = $1")
        .bind(project_id.as_uuid())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    sync_status(pool, project_id).await
}

/// Creates a deterministic `PostgreSQL` DDL plan for the current schema revision.
///
/// # Errors
/// Returns `NotFound` when the schema does not belong to the project.
pub async fn create_projection_plan(
    pool: &PgPool,
    id: ProjectionPlanId,
    project_id: ProjectId,
    schema_id: SchemaId,
    actor: UserId,
) -> Result<ProjectionPlan, RepositoryError> {
    let document: Value = sqlx::query_scalar(
        "SELECT document FROM datahub_schemas WHERE project_id = $1 AND id = $2",
    )
    .bind(project_id.as_uuid())
    .bind(schema_id.as_uuid())
    .fetch_optional(pool)
    .await?
    .ok_or(RepositoryError::NotFound)?;
    let previous: Option<Value> = sqlx::query_scalar(
        "SELECT schema_document FROM datahub_projection_schema_versions WHERE project_id = $1 AND schema_id = $2",
    )
    .bind(project_id.as_uuid())
    .bind(schema_id.as_uuid())
    .fetch_optional(pool)
    .await?;
    let operations = projection_operations(schema_id, previous.as_ref(), &document);
    let destructive = operations.iter().any(|operation| operation.destructive);
    sqlx::query(
        "INSERT INTO datahub_projection_plans (id, project_id, schema_id, status, destructive, operations, schema_document, created_by) VALUES ($1, $2, $3, 'draft', $4, $5, $6, $7)",
    )
    .bind(id.as_uuid())
    .bind(project_id.as_uuid())
    .bind(schema_id.as_uuid())
    .bind(destructive)
    .bind(serde_json::to_value(&operations)?)
    .bind(document)
    .bind(actor.as_uuid())
    .execute(pool)
    .await?;
    projection_plan(pool, project_id, id).await
}

/// Lists DDL plans newest first.
///
/// # Errors
/// Returns database errors.
pub async fn list_projection_plans(
    pool: &PgPool,
    project_id: ProjectId,
) -> Result<Vec<ProjectionPlan>, RepositoryError> {
    let rows = sqlx::query("SELECT id, schema_id, status, destructive, operations, approved_by FROM datahub_projection_plans WHERE project_id = $1 ORDER BY created_at DESC, id DESC")
        .bind(project_id.as_uuid())
        .fetch_all(pool)
        .await?;
    rows.iter()
        .map(|row| projection_plan_from_row(row, project_id))
        .collect()
}

async fn projection_plan(
    pool: &PgPool,
    project_id: ProjectId,
    id: ProjectionPlanId,
) -> Result<ProjectionPlan, RepositoryError> {
    let row = sqlx::query("SELECT id, schema_id, status, destructive, operations, approved_by FROM datahub_projection_plans WHERE project_id = $1 AND id = $2")
        .bind(project_id.as_uuid())
        .bind(id.as_uuid())
        .fetch_optional(pool)
        .await?
        .ok_or(RepositoryError::NotFound)?;
    projection_plan_from_row(&row, project_id)
}

fn projection_plan_from_row(
    row: &sqlx::postgres::PgRow,
    project_id: ProjectId,
) -> Result<ProjectionPlan, RepositoryError> {
    let operations: Value = row.try_get("operations")?;
    Ok(ProjectionPlan {
        id: ProjectionPlanId::from_uuid(row.try_get("id")?),
        project_id,
        schema_id: SchemaId::from_uuid(row.try_get("schema_id")?),
        status: row.try_get("status")?,
        destructive: row.try_get("destructive")?,
        operations: serde_json::from_value(operations)?,
        approved_by: row
            .try_get::<Option<Uuid>, _>("approved_by")?
            .map(UserId::from_uuid),
    })
}

/// Approves a projection plan. Approval is mandatory for destructive DDL.
///
/// # Errors
/// Returns `Conflict` unless the plan is still a draft.
pub async fn approve_projection_plan(
    pool: &PgPool,
    project_id: ProjectId,
    id: ProjectionPlanId,
    actor: UserId,
) -> Result<ProjectionPlan, RepositoryError> {
    let result = sqlx::query("UPDATE datahub_projection_plans SET status = 'approved', approved_by = $3, approved_at = NOW() WHERE project_id = $1 AND id = $2 AND status = 'draft'")
        .bind(project_id.as_uuid())
        .bind(id.as_uuid())
        .bind(actor.as_uuid())
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(RepositoryError::Conflict);
    }
    projection_plan(pool, project_id, id).await
}

/// Applies a generated DDL plan and records its schema snapshot atomically.
///
/// # Errors
/// Returns `Conflict` for an applied plan or an unapproved destructive plan.
pub async fn apply_projection_plan(
    pool: &PgPool,
    project_id: ProjectId,
    id: ProjectionPlanId,
    actor: UserId,
) -> Result<ProjectionPlan, RepositoryError> {
    let mut tx = pool.begin().await?;
    let row = sqlx::query("SELECT schema_id, status, destructive, operations, schema_document FROM datahub_projection_plans WHERE project_id = $1 AND id = $2 FOR UPDATE")
        .bind(project_id.as_uuid())
        .bind(id.as_uuid())
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(RepositoryError::NotFound)?;
    let status: String = row.try_get("status")?;
    let destructive: bool = row.try_get("destructive")?;
    if status == "applied" || (destructive && status != "approved") {
        return Err(RepositoryError::Conflict);
    }
    let operations: Vec<ProjectionOperation> = serde_json::from_value(row.try_get("operations")?)?;
    for operation in operations {
        sqlx::query(&operation.sql).execute(&mut *tx).await?;
    }
    let schema_id: Uuid = row.try_get("schema_id")?;
    let document: Value = row.try_get("schema_document")?;
    sqlx::query("INSERT INTO datahub_projection_schema_versions (project_id, schema_id, schema_document, plan_id) VALUES ($1, $2, $3, $4) ON CONFLICT (project_id, schema_id) DO UPDATE SET schema_document = EXCLUDED.schema_document, plan_id = EXCLUDED.plan_id, applied_at = NOW()")
        .bind(project_id.as_uuid())
        .bind(schema_id)
        .bind(document)
        .bind(id.as_uuid())
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE datahub_projection_plans SET status = 'applied', applied_at = NOW() WHERE id = $1",
    )
    .bind(id.as_uuid())
    .execute(&mut *tx)
    .await?;
    append_events(
        &mut tx,
        actor,
        Some(project_id),
        "projection.applied",
        "projection_plan",
        id.as_uuid(),
        &json!({"destructive": destructive}),
        &format!("projection-plan:{id}:apply"),
    )
    .await?;
    tx.commit().await?;
    projection_plan(pool, project_id, id).await
}

fn projection_operations(
    schema_id: SchemaId,
    previous: Option<&Value>,
    current: &Value,
) -> Vec<ProjectionOperation> {
    let table = format!("datahub_sync_{}", schema_id.to_string().replace('-', ""));
    let current_fields = projection_fields(current);
    let Some(previous) = previous else {
        let columns = current_fields
            .values()
            .map(|(name, ty)| format!("{name} {ty}"))
            .collect::<Vec<_>>()
            .join(", ");
        let suffix = if columns.is_empty() {
            String::new()
        } else {
            format!(", {columns}")
        };
        return vec![ProjectionOperation {
            sql: format!(
                "CREATE TABLE IF NOT EXISTS {table} (row_id UUID PRIMARY KEY, document JSONB NOT NULL{suffix})"
            ),
            destructive: false,
        }];
    };
    let previous_fields = projection_fields(previous);
    let mut operations = Vec::new();
    for (id, (name, ty)) in &current_fields {
        match previous_fields.get(id) {
            None => operations.push(ProjectionOperation {
                sql: format!("ALTER TABLE {table} ADD COLUMN IF NOT EXISTS {name} {ty}"),
                destructive: false,
            }),
            Some((_, old_ty)) if old_ty != ty => operations.push(ProjectionOperation {
                sql: format!(
                    "ALTER TABLE {table} ALTER COLUMN {name} TYPE {ty} USING {name}::{ty}"
                ),
                destructive: true,
            }),
            _ => {}
        }
    }
    for (id, (name, _)) in &previous_fields {
        if !current_fields.contains_key(id) {
            operations.push(ProjectionOperation {
                sql: format!("ALTER TABLE {table} DROP COLUMN IF EXISTS {name}"),
                destructive: true,
            });
        }
    }
    operations
}

fn projection_fields(document: &Value) -> std::collections::BTreeMap<String, (String, String)> {
    document
        .get("fields")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|field| {
            let id = field.get("id")?.as_str()?.replace('-', "");
            let ty = projection_pg_type(field.get("ty")?);
            Some((id.clone(), (format!("field_{id}"), ty)))
        })
        .collect()
}

fn projection_pg_type(ty: &Value) -> String {
    match ty.get("kind").and_then(Value::as_str) {
        Some("bool") => "BOOLEAN".into(),
        Some("integer") => "BIGINT".into(),
        Some("float") => "DOUBLE PRECISION".into(),
        Some("bytes") => "BYTEA".into(),
        Some("date") => "DATE".into(),
        Some("date_time") => "TIMESTAMPTZ".into(),
        Some("optional") => ty
            .get("item")
            .map_or_else(|| "JSONB".into(), projection_pg_type),
        Some("string") => "TEXT".into(),
        _ => "JSONB".into(),
    }
}

/// Creates a named deployment environment with an explicit approval policy.
///
/// # Errors
/// Returns a conflict for duplicate names.
pub async fn create_environment(
    pool: &PgPool,
    id: EnvironmentId,
    project_id: ProjectId,
    name: &str,
    requires_approval: bool,
) -> Result<EnvironmentRecord, RepositoryError> {
    let row = sqlx::query("INSERT INTO datahub_environments (id, project_id, name, requires_approval) VALUES ($1, $2, $3, $4) RETURNING id, name, requires_approval, current_release_id, version")
        .bind(id.as_uuid())
        .bind(project_id.as_uuid())
        .bind(name.trim())
        .bind(requires_approval)
        .fetch_one(pool)
        .await?;
    environment_from_row(&row, project_id)
}

/// Lists project deployment environments.
///
/// # Errors
/// Returns database errors.
pub async fn list_environments(
    pool: &PgPool,
    project_id: ProjectId,
) -> Result<Vec<EnvironmentRecord>, RepositoryError> {
    let rows = sqlx::query("SELECT id, name, requires_approval, current_release_id, version FROM datahub_environments WHERE project_id = $1 ORDER BY name, id")
        .bind(project_id.as_uuid())
        .fetch_all(pool)
        .await?;
    rows.iter()
        .map(|row| environment_from_row(row, project_id))
        .collect()
}

fn environment_from_row(
    row: &sqlx::postgres::PgRow,
    project_id: ProjectId,
) -> Result<EnvironmentRecord, RepositoryError> {
    Ok(EnvironmentRecord {
        id: EnvironmentId::from_uuid(row.try_get("id")?),
        project_id,
        name: row.try_get("name")?,
        requires_approval: row.try_get("requires_approval")?,
        current_release_id: row
            .try_get::<Option<Uuid>, _>("current_release_id")?
            .map(ReleaseId::from_uuid),
        version: row.try_get("version")?,
    })
}

/// Creates an immutable release snapshot from a successful deterministic build.
///
/// # Errors
/// Returns `NotFound` for a missing environment/build and `Conflict` for invalid build state.
#[allow(clippy::too_many_arguments)]
pub async fn create_release(
    pool: &PgPool,
    id: ReleaseId,
    project_id: ProjectId,
    environment_id: EnvironmentId,
    build_id: BuildId,
    version: &str,
    actor: UserId,
) -> Result<ReleaseRecord, RepositoryError> {
    let mut tx = pool.begin().await?;
    let environment_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM datahub_environments WHERE id = $1 AND project_id = $2)",
    )
    .bind(environment_id.as_uuid())
    .bind(project_id.as_uuid())
    .fetch_one(&mut *tx)
    .await?;
    if !environment_exists {
        return Err(RepositoryError::NotFound);
    }
    let build = sqlx::query("SELECT status, input_hash, manifest FROM datahub_jobs WHERE id = $1 AND project_id = $2 AND kind = 'build'")
        .bind(build_id.as_uuid())
        .bind(project_id.as_uuid())
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(RepositoryError::NotFound)?;
    let status: String = build.try_get("status")?;
    let input_hash: Option<String> = build.try_get("input_hash")?;
    let manifest: Option<Value> = build.try_get("manifest")?;
    let (Some(input_hash), Some(manifest)) = (input_hash, manifest) else {
        return Err(RepositoryError::Conflict);
    };
    if status != "succeeded" {
        return Err(RepositoryError::Conflict);
    }
    sqlx::query("INSERT INTO datahub_releases (id, project_id, environment_id, build_id, version, status, input_hash, manifest, created_by) VALUES ($1, $2, $3, $4, $5, 'draft', $6, $7, $8)")
        .bind(id.as_uuid())
        .bind(project_id.as_uuid())
        .bind(environment_id.as_uuid())
        .bind(build_id.as_uuid())
        .bind(version.trim())
        .bind(input_hash)
        .bind(manifest)
        .bind(actor.as_uuid())
        .execute(&mut *tx)
        .await?;
    append_events(
        &mut tx,
        actor,
        Some(project_id),
        "release.created",
        "release",
        id.as_uuid(),
        &json!({"environment_id": environment_id, "build_id": build_id, "version": version}),
        &format!("release:{id}:create"),
    )
    .await?;
    tx.commit().await?;
    release(pool, project_id, id).await
}

/// Lists immutable release snapshots.
///
/// # Errors
/// Returns database errors.
pub async fn list_releases(
    pool: &PgPool,
    project_id: ProjectId,
) -> Result<Vec<ReleaseRecord>, RepositoryError> {
    let rows = sqlx::query("SELECT id, environment_id, build_id, version, status, input_hash, manifest, approved_by, rollback_of FROM datahub_releases WHERE project_id = $1 ORDER BY created_at DESC, id DESC")
        .bind(project_id.as_uuid())
        .fetch_all(pool)
        .await?;
    rows.iter()
        .map(|row| release_from_row(row, project_id))
        .collect()
}

async fn release(
    pool: &PgPool,
    project_id: ProjectId,
    id: ReleaseId,
) -> Result<ReleaseRecord, RepositoryError> {
    let row = sqlx::query("SELECT id, environment_id, build_id, version, status, input_hash, manifest, approved_by, rollback_of FROM datahub_releases WHERE project_id = $1 AND id = $2")
        .bind(project_id.as_uuid())
        .bind(id.as_uuid())
        .fetch_optional(pool)
        .await?
        .ok_or(RepositoryError::NotFound)?;
    release_from_row(&row, project_id)
}

fn release_from_row(
    row: &sqlx::postgres::PgRow,
    project_id: ProjectId,
) -> Result<ReleaseRecord, RepositoryError> {
    Ok(ReleaseRecord {
        id: ReleaseId::from_uuid(row.try_get("id")?),
        project_id,
        environment_id: EnvironmentId::from_uuid(row.try_get("environment_id")?),
        build_id: BuildId::from_uuid(row.try_get("build_id")?),
        version: row.try_get("version")?,
        status: row.try_get("status")?,
        input_hash: row.try_get("input_hash")?,
        manifest: row.try_get("manifest")?,
        approved_by: row
            .try_get::<Option<Uuid>, _>("approved_by")?
            .map(UserId::from_uuid),
        rollback_of: row
            .try_get::<Option<Uuid>, _>("rollback_of")?
            .map(ReleaseId::from_uuid),
    })
}

/// Approves a draft release.
///
/// # Errors
/// Returns `Conflict` when the release is not a draft.
pub async fn approve_release(
    pool: &PgPool,
    project_id: ProjectId,
    id: ReleaseId,
    actor: UserId,
) -> Result<ReleaseRecord, RepositoryError> {
    let result = sqlx::query("UPDATE datahub_releases SET status = 'approved', approved_by = $3, approved_at = NOW() WHERE project_id = $1 AND id = $2 AND status = 'draft'")
        .bind(project_id.as_uuid())
        .bind(id.as_uuid())
        .bind(actor.as_uuid())
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(RepositoryError::Conflict);
    }
    release(pool, project_id, id).await
}

/// Publishes a release and atomically advances its environment pointer.
///
/// # Errors
/// Returns `Conflict` when policy or state disallows publication.
pub async fn publish_release(
    pool: &PgPool,
    project_id: ProjectId,
    id: ReleaseId,
    actor: UserId,
) -> Result<ReleaseRecord, RepositoryError> {
    let mut tx = pool.begin().await?;
    let row = sqlx::query("SELECT r.environment_id, r.status, e.requires_approval FROM datahub_releases r JOIN datahub_environments e ON e.id = r.environment_id WHERE r.project_id = $1 AND r.id = $2 FOR UPDATE OF r, e")
        .bind(project_id.as_uuid())
        .bind(id.as_uuid())
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(RepositoryError::NotFound)?;
    let status: String = row.try_get("status")?;
    let requires_approval: bool = row.try_get("requires_approval")?;
    if status == "published" || (requires_approval && status != "approved") {
        return Err(RepositoryError::Conflict);
    }
    let environment_id: Uuid = row.try_get("environment_id")?;
    sqlx::query("UPDATE datahub_releases SET status = 'superseded' WHERE environment_id = $1 AND status = 'published'")
        .bind(environment_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE datahub_releases SET status = 'published', published_at = NOW() WHERE id = $1",
    )
    .bind(id.as_uuid())
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE datahub_environments SET current_release_id = $2, version = version + 1 WHERE id = $1")
        .bind(environment_id)
        .bind(id.as_uuid())
        .execute(&mut *tx)
        .await?;
    append_events(
        &mut tx,
        actor,
        Some(project_id),
        "release.published",
        "release",
        id.as_uuid(),
        &json!({"environment_id": environment_id}),
        &format!("release:{id}:publish"),
    )
    .await?;
    tx.commit().await?;
    release(pool, project_id, id).await
}

/// Rolls an environment back by publishing a new immutable release snapshot of a historical one.
///
/// # Errors
/// Returns `NotFound` for a release outside the environment.
pub async fn rollback_release(
    pool: &PgPool,
    project_id: ProjectId,
    environment_id: EnvironmentId,
    target: ReleaseId,
    id: ReleaseId,
    version: &str,
    actor: UserId,
) -> Result<ReleaseRecord, RepositoryError> {
    let mut tx = pool.begin().await?;
    let target_row = sqlx::query("SELECT build_id, input_hash, manifest FROM datahub_releases WHERE project_id = $1 AND environment_id = $2 AND id = $3 AND status IN ('published', 'superseded')")
        .bind(project_id.as_uuid())
        .bind(environment_id.as_uuid())
        .bind(target.as_uuid())
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(RepositoryError::NotFound)?;
    sqlx::query("UPDATE datahub_releases SET status = 'superseded' WHERE environment_id = $1 AND status = 'published'")
        .bind(environment_id.as_uuid())
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO datahub_releases (id, project_id, environment_id, build_id, version, status, input_hash, manifest, created_by, approved_by, rollback_of, approved_at, published_at) VALUES ($1, $2, $3, $4, $5, 'published', $6, $7, $8, $8, $9, NOW(), NOW())")
        .bind(id.as_uuid())
        .bind(project_id.as_uuid())
        .bind(environment_id.as_uuid())
        .bind(target_row.try_get::<Uuid, _>("build_id")?)
        .bind(version.trim())
        .bind(target_row.try_get::<String, _>("input_hash")?)
        .bind(target_row.try_get::<Value, _>("manifest")?)
        .bind(actor.as_uuid())
        .bind(target.as_uuid())
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE datahub_environments SET current_release_id = $2, version = version + 1 WHERE id = $1")
        .bind(environment_id.as_uuid())
        .bind(id.as_uuid())
        .execute(&mut *tx)
        .await?;
    append_events(
        &mut tx,
        actor,
        Some(project_id),
        "release.rolled_back",
        "release",
        id.as_uuid(),
        &json!({"environment_id": environment_id, "rollback_of": target}),
        &format!("release:{id}:rollback"),
    )
    .await?;
    tx.commit().await?;
    release(pool, project_id, id).await
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

fn formula_set_from_row(row: &sqlx::postgres::PgRow) -> Result<StoredFormulaSet, RepositoryError> {
    Ok(StoredFormulaSet {
        schema_id: SchemaId::from_uuid(row.try_get("schema_id")?),
        schema_revision_id: RevisionId::from_uuid(row.try_get("schema_revision_id")?),
        document: row.try_get("document")?,
        version: row.try_get("version")?,
        revision_id: RevisionId::from_uuid(row.try_get("current_revision_id")?),
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

#[cfg(test)]
mod tests {
    use datahub_kernel::SchemaId;
    use serde_json::json;

    use super::projection_operations;

    #[test]
    fn initial_projection_plan_is_compatible_and_stable_id_based() {
        let schema_id = SchemaId::new();
        let field_id = uuid::Uuid::now_v7();
        let document = json!({
            "fields": [{"id": field_id, "ty": {"kind": "integer"}}]
        });
        let operations = projection_operations(schema_id, None, &document);
        assert_eq!(operations.len(), 1);
        assert!(!operations[0].destructive);
        assert!(operations[0].sql.contains("CREATE TABLE IF NOT EXISTS"));
        assert!(
            operations[0]
                .sql
                .contains(&field_id.to_string().replace('-', ""))
        );
        assert!(!operations[0].sql.contains('"'));
    }

    #[test]
    fn removed_or_retyped_fields_require_destructive_approval() {
        let schema_id = SchemaId::new();
        let removed = uuid::Uuid::now_v7();
        let retyped = uuid::Uuid::now_v7();
        let previous = json!({
            "fields": [
                {"id": removed, "ty": {"kind": "string"}},
                {"id": retyped, "ty": {"kind": "integer"}}
            ]
        });
        let current = json!({
            "fields": [{"id": retyped, "ty": {"kind": "string"}}]
        });
        let operations = projection_operations(schema_id, Some(&previous), &current);
        assert_eq!(operations.len(), 2);
        assert!(operations.iter().all(|operation| operation.destructive));
        assert!(
            operations
                .iter()
                .any(|operation| operation.sql.contains("DROP COLUMN"))
        );
        assert!(
            operations
                .iter()
                .any(|operation| operation.sql.contains("ALTER COLUMN"))
        );
    }
}
