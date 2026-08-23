use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::HeaderMap,
    routing::{get, post, put},
};
use datahub_auth::{digest_token, hash_password, issue_token, verify_password};
use datahub_export::{
    ExportError, generate_code_for_audience, generate_csv_for_audience, generate_json_for_audience,
};
use datahub_formula::{
    EvaluationRuntime, FormulaDefinition, FormulaSet, evaluate_formulas, parse_formula,
};
use datahub_kernel::{
    Audience, BuildId, CompilationTarget, ConfigRow, ConfigValue, FieldId, ProjectAction,
    ProjectId, ProjectRole, RevisionId, RowId, SchemaDefinition, SchemaId, SessionId, TableViewId,
    UserId, validate_row, validate_schema,
};
use datahub_persistence_pg::{
    BuildArtifact, BuildRecord, ProjectRecord, RowWrite, SessionPrincipal, StoredFormulaSet,
    StoredRow, StoredSchema, SyncStatus, UserAccount, add_project_member, create_initial_user,
    create_project, create_session, create_user, list_builds, list_projects, list_rows,
    list_schemas, load_formula_set, project_role, record_build, row_exists, save_formula_set,
    save_row, save_rows_atomic, save_schema, session_principal, sync_status, user_by_username,
    user_count,
};
use datahub_xlsx::{VersionedRow, XlsxError, export_workbook, import_workbook};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Postgres, QueryBuilder, Row};

use crate::{AppState, error::ApiError};

const SESSION_TTL_SECONDS: i32 = 12 * 60 * 60;
const XLSX_BODY_LIMIT: usize = 64 * 1024 * 1024;

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/setup", get(setup_status))
        .route("/auth/bootstrap", post(bootstrap))
        .route("/auth/login", post(login))
        .route("/me", get(me))
        .route("/users", post(create_user_handler))
        .route("/projects", get(projects).post(create_project_handler))
        .route(
            "/projects/{project_id}/members/{user_id}",
            put(update_member_handler),
        )
        .route(
            "/projects/{project_id}/schemas",
            get(schemas).post(create_schema_handler),
        )
        .route(
            "/projects/{project_id}/schemas/{schema_id}",
            put(update_schema_handler),
        )
        .route(
            "/projects/{project_id}/schemas/{schema_id}/rows",
            get(rows).post(create_row_handler),
        )
        .route(
            "/projects/{project_id}/schemas/{schema_id}/rows/{row_id}",
            put(update_row_handler),
        )
        .route(
            "/projects/{project_id}/schemas/{schema_id}/formulas",
            get(formulas).put(save_formulas),
        )
        .route(
            "/projects/{project_id}/schemas/{schema_id}/formulas/preview",
            post(preview_formulas),
        )
        .route(
            "/projects/{project_id}/schemas/{schema_id}/formulas/apply",
            post(apply_formulas),
        )
        .route(
            "/projects/{project_id}/schemas/{schema_id}/xlsx/export",
            post(export_xlsx),
        )
        .route(
            "/projects/{project_id}/schemas/{schema_id}/xlsx/preview",
            post(preview_xlsx).layer(DefaultBodyLimit::max(XLSX_BODY_LIMIT)),
        )
        .route(
            "/projects/{project_id}/schemas/{schema_id}/xlsx/commit",
            post(commit_xlsx).layer(DefaultBodyLimit::max(XLSX_BODY_LIMIT)),
        )
        .route(
            "/projects/{project_id}/schemas/{schema_id}/views",
            post(create_table_view),
        )
        .route(
            "/table-views/{view_id}/blocks/{block_index}",
            get(table_view_block),
        )
        .route(
            "/projects/{project_id}/builds",
            get(builds).post(create_build_handler),
        )
        .route(
            "/projects/{project_id}/sync-status",
            get(sync_status_handler),
        )
}

#[derive(Serialize)]
struct SetupStatus {
    requires_bootstrap: bool,
}

async fn setup_status(State(state): State<AppState>) -> Result<Json<SetupStatus>, ApiError> {
    Ok(Json(SetupStatus {
        requires_bootstrap: user_count(&state.pool).await? == 0,
    }))
}

#[derive(Deserialize)]
struct Credentials {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct AuthResponse {
    user: UserAccount,
    token: String,
    csrf_token: String,
    expires_in: i32,
}

async fn bootstrap(
    State(state): State<AppState>,
    Json(credentials): Json<Credentials>,
) -> Result<Json<AuthResponse>, ApiError> {
    validate_username(&credentials.username)?;
    let password_hash = hash_password(&credentials.password)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let user = create_initial_user(
        &state.pool,
        UserId::new(),
        &credentials.username,
        &password_hash,
    )
    .await
    .map_err(|error| match error {
        datahub_persistence_pg::RepositoryError::Conflict => {
            ApiError::conflict("DataHub has already been bootstrapped")
        }
        other => other.into(),
    })?;
    create_auth_response(&state, user).await.map(Json)
}

async fn login(
    State(state): State<AppState>,
    Json(credentials): Json<Credentials>,
) -> Result<Json<AuthResponse>, ApiError> {
    let user = user_by_username(&state.pool, &credentials.username)
        .await?
        .filter(|user| verify_password(&credentials.password, &user.password_hash))
        .ok_or_else(ApiError::unauthorized)?;
    create_auth_response(&state, user).await.map(Json)
}

async fn create_auth_response(
    state: &AppState,
    user: UserAccount,
) -> Result<AuthResponse, ApiError> {
    let token = issue_token();
    let csrf = issue_token();
    create_session(
        &state.pool,
        SessionId::new(),
        user.id,
        &token.digest,
        &csrf.digest,
        SESSION_TTL_SECONDS,
    )
    .await?;
    Ok(AuthResponse {
        user,
        token: token.plaintext,
        csrf_token: csrf.plaintext,
        expires_in: SESSION_TTL_SECONDS,
    })
}

#[derive(Serialize)]
struct MeResponse {
    id: UserId,
    username: String,
    is_system_admin: bool,
}

async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MeResponse>, ApiError> {
    let principal = authenticate(&state, &headers, false).await?;
    Ok(Json(MeResponse {
        id: principal.user_id,
        username: principal.username,
        is_system_admin: principal.is_system_admin,
    }))
}

async fn create_user_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(credentials): Json<Credentials>,
) -> Result<Json<UserAccount>, ApiError> {
    let principal = authenticate(&state, &headers, true).await?;
    if !principal.is_system_admin {
        return Err(ApiError::forbidden(
            "system administrator permission is required",
        ));
    }
    validate_username(&credentials.username)?;
    let password_hash = hash_password(&credentials.password)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    Ok(Json(
        create_user(
            &state.pool,
            UserId::new(),
            &credentials.username,
            &password_hash,
        )
        .await?,
    ))
}

async fn projects(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ProjectRecord>>, ApiError> {
    let principal = authenticate(&state, &headers, false).await?;
    Ok(Json(list_projects(&state.pool, principal.user_id).await?))
}

#[derive(Deserialize)]
struct CreateProjectRequest {
    name: String,
    #[serde(default)]
    description: String,
}

async fn create_project_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateProjectRequest>,
) -> Result<Json<ProjectRecord>, ApiError> {
    let principal = authenticate(&state, &headers, true).await?;
    if request.name.trim().is_empty() {
        return Err(ApiError::bad_request("project name cannot be empty"));
    }
    Ok(Json(
        create_project(
            &state.pool,
            ProjectId::new(),
            principal.user_id,
            &request.name,
            &request.description,
        )
        .await?,
    ))
}

#[derive(Deserialize)]
struct UpdateMemberRequest {
    role: ProjectRole,
}

async fn update_member_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, user_id)): Path<(ProjectId, UserId)>,
    Json(request): Json<UpdateMemberRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let principal = authenticate(&state, &headers, true).await?;
    authorize_project(
        &state,
        project_id,
        principal.user_id,
        ProjectAction::ManageMembers,
    )
    .await?;
    add_project_member(
        &state.pool,
        project_id,
        user_id,
        request.role,
        principal.user_id,
    )
    .await?;
    Ok(Json(json!({"updated": true})))
}

async fn schemas(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<ProjectId>,
) -> Result<Json<Vec<StoredSchema>>, ApiError> {
    let principal = authenticate(&state, &headers, false).await?;
    authorize_project(&state, project_id, principal.user_id, ProjectAction::Read).await?;
    Ok(Json(list_schemas(&state.pool, project_id).await?))
}

#[derive(Deserialize)]
struct SaveSchemaRequest {
    definition: SchemaDefinition,
    expected_version: Option<i64>,
}

async fn create_schema_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<ProjectId>,
    Json(mut request): Json<SaveSchemaRequest>,
) -> Result<Json<StoredSchema>, ApiError> {
    request.expected_version = None;
    save_schema_handler(state, headers, project_id, request).await
}

async fn update_schema_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, schema_id)): Path<(ProjectId, SchemaId)>,
    Json(request): Json<SaveSchemaRequest>,
) -> Result<Json<StoredSchema>, ApiError> {
    if request.definition.id != schema_id {
        return Err(ApiError::bad_request(
            "schema id does not match request path",
        ));
    }
    if request.expected_version.is_none() {
        return Err(ApiError::bad_request(
            "expected_version is required when updating a schema",
        ));
    }
    save_schema_handler(state, headers, project_id, request).await
}

async fn save_schema_handler(
    state: AppState,
    headers: HeaderMap,
    project_id: ProjectId,
    mut request: SaveSchemaRequest,
) -> Result<Json<StoredSchema>, ApiError> {
    let principal = authenticate(&state, &headers, true).await?;
    authorize_project(&state, project_id, principal.user_id, ProjectAction::Write).await?;
    request.definition.project_id = project_id;
    let issues = validate_schema(&request.definition);
    if !issues.is_empty() {
        return Err(ApiError::validation(json!(issues)));
    }
    Ok(Json(
        save_schema(
            &state.pool,
            &request.definition,
            principal.user_id,
            request.expected_version,
        )
        .await?,
    ))
}

fn collect_references(value: &ConfigValue, references: &mut Vec<(SchemaId, RowId)>) {
    match value {
        ConfigValue::Reference { schema_id, row_id } => references.push((*schema_id, *row_id)),
        ConfigValue::List(values) | ConfigValue::Set(values) | ConfigValue::FixedArray(values) => {
            for value in values {
                collect_references(value, references);
            }
        }
        ConfigValue::Map(values) => {
            for value in values.values() {
                collect_references(value, references);
            }
        }
        ConfigValue::Struct(values) => {
            for value in values.values() {
                collect_references(value, references);
            }
        }
        ConfigValue::Union { value, .. } | ConfigValue::Custom { value, .. } => {
            collect_references(value, references);
        }
        ConfigValue::Null
        | ConfigValue::Bool(_)
        | ConfigValue::Integer(_)
        | ConfigValue::Float(_)
        | ConfigValue::String(_)
        | ConfigValue::Bytes(_)
        | ConfigValue::Date(_)
        | ConfigValue::DateTime(_)
        | ConfigValue::Enum(_) => {}
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ViewFilter {
    field_id: FieldId,
    value: ConfigValue,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ViewSort {
    field_id: FieldId,
    direction: SortDirection,
}

#[derive(Deserialize)]
struct CreateTableViewRequest {
    #[serde(default = "default_block_size")]
    block_size: i32,
    #[serde(default)]
    sort: Vec<ViewSort>,
    #[serde(default)]
    filters: Vec<ViewFilter>,
}

const fn default_block_size() -> i32 {
    512
}

#[derive(Serialize)]
struct TableViewResponse {
    view_id: TableViewId,
    total_rows: i64,
    block_size: i32,
    data_revision: Option<RevisionId>,
}

async fn create_table_view(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, schema_id)): Path<(ProjectId, SchemaId)>,
    Json(request): Json<CreateTableViewRequest>,
) -> Result<Json<TableViewResponse>, ApiError> {
    let principal = authenticate(&state, &headers, true).await?;
    authorize_project(&state, project_id, principal.user_id, ProjectAction::Read).await?;
    if !(256..=1024).contains(&request.block_size) {
        return Err(ApiError::bad_request(
            "table view block_size must be between 256 and 1024",
        ));
    }
    let total_rows = count_view_rows(&state.pool, schema_id, &request.filters).await?;
    let data_revision = sqlx::query_scalar(
        "SELECT revision_id FROM datahub_data_revisions WHERE project_id = $1 AND schema_id = $2 ORDER BY created_at DESC, revision_id DESC LIMIT 1",
    )
    .bind(project_id.as_uuid())
    .bind(schema_id.as_uuid())
    .fetch_optional(&state.pool)
    .await?
    .map(RevisionId::from_uuid);
    let view_id = TableViewId::new();
    sqlx::query(
        "INSERT INTO datahub_table_views (id, project_id, schema_id, created_by, block_size, sort_spec, filter_spec, data_revision_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(view_id.as_uuid())
    .bind(project_id.as_uuid())
    .bind(schema_id.as_uuid())
    .bind(principal.user_id.as_uuid())
    .bind(request.block_size)
    .bind(json!(request.sort))
    .bind(json!(request.filters))
    .bind(data_revision.map(RevisionId::as_uuid))
    .execute(&state.pool)
    .await?;
    Ok(Json(TableViewResponse {
        view_id,
        total_rows,
        block_size: request.block_size,
        data_revision,
    }))
}

async fn table_view_block(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((view_id, block_index)): Path<(TableViewId, i64)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if block_index < 0 {
        return Err(ApiError::bad_request("block index cannot be negative"));
    }
    let principal = authenticate(&state, &headers, false).await?;
    let view = sqlx::query(
        "SELECT project_id, schema_id, block_size, sort_spec, filter_spec, data_revision_id FROM datahub_table_views WHERE id = $1 AND expires_at > NOW()",
    )
    .bind(view_id.as_uuid())
    .fetch_optional(&state.pool)
    .await?
    .ok_or(datahub_persistence_pg::RepositoryError::NotFound)?;
    let project_id = ProjectId::from_uuid(view.try_get("project_id")?);
    authorize_project(&state, project_id, principal.user_id, ProjectAction::Read).await?;
    let schema_id = SchemaId::from_uuid(view.try_get("schema_id")?);
    let block_size: i32 = view.try_get("block_size")?;
    let sort: Vec<ViewSort> = serde_json::from_value(view.try_get("sort_spec")?)
        .map_err(|_| ApiError::internal("stored table view sort is invalid"))?;
    let filters: Vec<ViewFilter> = serde_json::from_value(view.try_get("filter_spec")?)
        .map_err(|_| ApiError::internal("stored table view filter is invalid"))?;
    let offset = block_index
        .checked_mul(i64::from(block_size))
        .ok_or_else(|| ApiError::bad_request("block offset overflow"))?;
    let rows = fetch_view_rows(&state.pool, schema_id, &filters, &sort, block_size, offset).await?;
    let data_revision: Option<uuid::Uuid> = view.try_get("data_revision_id")?;
    Ok(Json(json!({
        "view_id": view_id,
        "block_index": block_index,
        "data_revision": data_revision.map(RevisionId::from_uuid),
        "rows": rows,
    })))
}

async fn count_view_rows(
    pool: &sqlx::PgPool,
    schema_id: SchemaId,
    filters: &[ViewFilter],
) -> Result<i64, ApiError> {
    let mut query = QueryBuilder::<Postgres>::new(
        "SELECT COUNT(*) AS total FROM datahub_config_rows WHERE schema_id = ",
    );
    query.push_bind(schema_id.as_uuid());
    push_view_filters(&mut query, filters);
    let row = query.build().fetch_one(pool).await?;
    Ok(row.try_get("total")?)
}

async fn fetch_view_rows(
    pool: &sqlx::PgPool,
    schema_id: SchemaId,
    filters: &[ViewFilter],
    sort: &[ViewSort],
    limit: i32,
    offset: i64,
) -> Result<Vec<serde_json::Value>, ApiError> {
    let mut query = QueryBuilder::<Postgres>::new(
        "SELECT document, version FROM datahub_config_rows WHERE schema_id = ",
    );
    query.push_bind(schema_id.as_uuid());
    push_view_filters(&mut query, filters);
    query.push(" ORDER BY ");
    for (index, item) in sort.iter().enumerate() {
        if index > 0 {
            query.push(", ");
        }
        query.push("document->'values'->");
        query.push_bind(item.field_id.to_string());
        query.push("->'value' ");
        query.push(match item.direction {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        });
    }
    if !sort.is_empty() {
        query.push(", ");
    }
    query.push("id ASC LIMIT ");
    query.push_bind(limit);
    query.push(" OFFSET ");
    query.push_bind(offset);
    let rows = query.build().fetch_all(pool).await?;
    rows.iter()
        .map(|row| {
            let document: serde_json::Value = row.try_get("document")?;
            let version: i64 = row.try_get("version")?;
            Ok(json!({"row": document, "version": version}))
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(ApiError::from)
}

fn push_view_filters<'a>(query: &mut QueryBuilder<'a, Postgres>, filters: &'a [ViewFilter]) {
    for filter in filters {
        query.push(" AND document->'values'->");
        query.push_bind(filter.field_id.to_string());
        query.push(" = ");
        query.push_bind(json!(filter.value));
    }
}

async fn rows(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, schema_id)): Path<(ProjectId, SchemaId)>,
) -> Result<Json<Vec<StoredRow>>, ApiError> {
    let principal = authenticate(&state, &headers, false).await?;
    authorize_project(&state, project_id, principal.user_id, ProjectAction::Read).await?;
    Ok(Json(list_rows(&state.pool, schema_id).await?))
}

#[derive(Deserialize)]
struct SaveRowRequest {
    row: ConfigRow,
    expected_version: Option<i64>,
}

async fn create_row_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, schema_id)): Path<(ProjectId, SchemaId)>,
    Json(mut request): Json<SaveRowRequest>,
) -> Result<Json<StoredRow>, ApiError> {
    request.expected_version = None;
    save_row_handler(state, headers, project_id, schema_id, request).await
}

async fn update_row_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, schema_id, row_id)): Path<(ProjectId, SchemaId, datahub_kernel::RowId)>,
    Json(request): Json<SaveRowRequest>,
) -> Result<Json<StoredRow>, ApiError> {
    if request.row.id != row_id || request.expected_version.is_none() {
        return Err(ApiError::bad_request(
            "row id must match the path and expected_version is required",
        ));
    }
    save_row_handler(state, headers, project_id, schema_id, request).await
}

async fn save_row_handler(
    state: AppState,
    headers: HeaderMap,
    project_id: ProjectId,
    schema_id: SchemaId,
    mut request: SaveRowRequest,
) -> Result<Json<StoredRow>, ApiError> {
    let principal = authenticate(&state, &headers, true).await?;
    authorize_project(&state, project_id, principal.user_id, ProjectAction::Write).await?;
    request.row.schema_id = schema_id;
    let schema = list_schemas(&state.pool, project_id)
        .await?
        .into_iter()
        .find(|stored| stored.definition.id == schema_id)
        .ok_or(datahub_persistence_pg::RepositoryError::NotFound)?;
    let issues = validate_row(&schema.definition, &request.row);
    if !issues.is_empty() {
        return Err(ApiError::validation(json!(issues)));
    }
    let mut references = Vec::new();
    for value in request.row.values.values() {
        collect_references(value, &mut references);
    }
    for (referenced_schema, referenced_row) in references {
        if !row_exists(&state.pool, referenced_schema, referenced_row).await? {
            return Err(ApiError::validation(json!([{
                "code": "reference_not_found",
                "path": "row.values",
                "message": format!("referenced row {referenced_row} does not exist in schema {referenced_schema}"),
            }])));
        }
    }
    Ok(Json(
        save_row(
            &state.pool,
            &request.row,
            principal.user_id,
            project_id,
            request.expected_version,
        )
        .await?,
    ))
}

#[derive(Deserialize)]
struct FormulaInput {
    field_id: FieldId,
    source: String,
}

#[derive(Deserialize)]
struct SaveFormulaSetRequest {
    definitions: Vec<FormulaInput>,
    expected_version: Option<i64>,
}

#[derive(Deserialize)]
struct FormulaRunRequest {
    #[serde(default = "default_formula_runtime")]
    runtime: EvaluationRuntime,
}

const fn default_formula_runtime() -> EvaluationRuntime {
    EvaluationRuntime::Native
}

#[derive(Serialize)]
struct FormulaChange {
    row_id: RowId,
    expected_version: i64,
    before: ConfigRow,
    after: ConfigRow,
}

async fn formulas(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, schema_id)): Path<(ProjectId, SchemaId)>,
) -> Result<Json<Option<StoredFormulaSet>>, ApiError> {
    let principal = authenticate(&state, &headers, false).await?;
    authorize_project(&state, project_id, principal.user_id, ProjectAction::Read).await?;
    Ok(Json(load_formula_set(&state.pool, schema_id).await?))
}

async fn save_formulas(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, schema_id)): Path<(ProjectId, SchemaId)>,
    Json(request): Json<SaveFormulaSetRequest>,
) -> Result<Json<StoredFormulaSet>, ApiError> {
    let principal = authenticate(&state, &headers, true).await?;
    authorize_project(&state, project_id, principal.user_id, ProjectAction::Write).await?;
    let schema = current_schema(&state, project_id, schema_id).await?;
    let existing = load_formula_set(&state.pool, schema_id).await?;
    match (&existing, request.expected_version) {
        (Some(_), None) => {
            return Err(ApiError::bad_request(
                "expected_version is required when updating formulas",
            ));
        }
        (None, Some(_)) => return Err(ApiError::conflict("formula set does not exist")),
        _ => {}
    }
    let formula_set = build_formula_set(&schema.definition, request.definitions)?;
    let document = serde_json::to_value(formula_set)
        .map_err(|_| ApiError::internal("formula serialization failed"))?;
    Ok(Json(
        save_formula_set(
            &state.pool,
            project_id,
            schema_id,
            schema.revision_id,
            &document,
            principal.user_id,
            request.expected_version,
        )
        .await?,
    ))
}

async fn preview_formulas(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, schema_id)): Path<(ProjectId, SchemaId)>,
    Json(request): Json<FormulaRunRequest>,
) -> Result<Json<Vec<FormulaChange>>, ApiError> {
    let principal = authenticate(&state, &headers, false).await?;
    authorize_project(&state, project_id, principal.user_id, ProjectAction::Read).await?;
    let schema = current_schema(&state, project_id, schema_id).await?;
    let formula_set = current_formula_set(&state, schema_id, schema.revision_id).await?;
    let rows = list_rows(&state.pool, schema_id).await?;
    Ok(Json(compute_formula_changes(
        &schema.definition,
        &formula_set,
        rows,
        request.runtime,
    )?))
}

async fn apply_formulas(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, schema_id)): Path<(ProjectId, SchemaId)>,
    Json(request): Json<FormulaRunRequest>,
) -> Result<Json<Vec<StoredRow>>, ApiError> {
    let principal = authenticate(&state, &headers, true).await?;
    authorize_project(&state, project_id, principal.user_id, ProjectAction::Write).await?;
    let schema = current_schema(&state, project_id, schema_id).await?;
    let formula_set = current_formula_set(&state, schema_id, schema.revision_id).await?;
    let rows = list_rows(&state.pool, schema_id).await?;
    let writes = compute_formula_changes(&schema.definition, &formula_set, rows, request.runtime)?
        .into_iter()
        .map(|change| RowWrite {
            row: change.after,
            expected_version: Some(change.expected_version),
        })
        .collect::<Vec<_>>();
    Ok(Json(
        save_rows_atomic(
            &state.pool,
            &writes,
            principal.user_id,
            project_id,
            "formula.applied",
        )
        .await?,
    ))
}

fn build_formula_set(
    schema: &SchemaDefinition,
    inputs: Vec<FormulaInput>,
) -> Result<FormulaSet, ApiError> {
    if inputs.len() > 256 || inputs.iter().any(|input| input.source.len() > 4096) {
        return Err(ApiError::bad_request(
            "a formula set supports at most 256 formulas of 4096 bytes each",
        ));
    }
    let definitions = inputs
        .into_iter()
        .map(|input| {
            if !schema.fields.iter().any(|field| field.id == input.field_id) {
                return Err(ApiError::validation(
                    json!({"formula": "target field does not exist"}),
                ));
            }
            let expression = parse_formula(&input.source, schema).map_err(formula_error)?;
            Ok(FormulaDefinition {
                field_id: input.field_id,
                source: input.source,
                expression,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    FormulaSet::from_definitions(definitions).map_err(formula_error)
}

async fn current_formula_set(
    state: &AppState,
    schema_id: SchemaId,
    schema_revision_id: RevisionId,
) -> Result<FormulaSet, ApiError> {
    let stored = load_formula_set(&state.pool, schema_id)
        .await?
        .ok_or(datahub_persistence_pg::RepositoryError::NotFound)?;
    if stored.schema_revision_id != schema_revision_id {
        return Err(ApiError::conflict(
            "formula set must be saved against the current schema revision",
        ));
    }
    serde_json::from_value(stored.document)
        .map_err(|_| ApiError::internal("stored formula set is invalid"))
}

fn compute_formula_changes(
    schema: &SchemaDefinition,
    formulas: &FormulaSet,
    rows: Vec<StoredRow>,
    runtime: EvaluationRuntime,
) -> Result<Vec<FormulaChange>, ApiError> {
    let mut changes = Vec::new();
    for stored in rows {
        let results =
            evaluate_formulas(formulas, &stored.row.values, runtime).map_err(formula_error)?;
        let mut after = stored.row.clone();
        for (field_id, value) in results {
            let field = schema
                .fields
                .iter()
                .find(|field| field.id == field_id)
                .ok_or_else(|| {
                    ApiError::validation(json!({"formula": "target field does not exist"}))
                })?;
            after
                .values
                .insert(field_id, value.to_config(&field.ty).map_err(formula_error)?);
        }
        if after.values != stored.row.values {
            changes.push(FormulaChange {
                row_id: stored.row.id,
                expected_version: stored.version,
                before: stored.row,
                after,
            });
        }
    }
    Ok(changes)
}

#[derive(Serialize)]
struct XlsxArtifact {
    file_name: String,
    content_type: &'static str,
    content: Vec<u8>,
}

#[derive(Deserialize)]
struct XlsxPayload {
    content: Vec<u8>,
}

#[derive(Serialize)]
struct XlsxPreview {
    created: usize,
    updated: usize,
    rows: Vec<datahub_xlsx::ImportedRow>,
}

async fn export_xlsx(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, schema_id)): Path<(ProjectId, SchemaId)>,
) -> Result<Json<XlsxArtifact>, ApiError> {
    let principal = authenticate(&state, &headers, false).await?;
    authorize_project(&state, project_id, principal.user_id, ProjectAction::Read).await?;
    let schema = current_schema(&state, project_id, schema_id).await?;
    let rows = list_rows(&state.pool, schema_id)
        .await?
        .into_iter()
        .map(|stored| VersionedRow {
            row: stored.row,
            version: stored.version,
        })
        .collect::<Vec<_>>();
    let content =
        export_workbook(&schema.definition, schema.revision_id, &rows).map_err(xlsx_error)?;
    Ok(Json(XlsxArtifact {
        file_name: format!("{}.xlsx", schema.definition.name),
        content_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        content,
    }))
}

async fn preview_xlsx(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, schema_id)): Path<(ProjectId, SchemaId)>,
    Json(payload): Json<XlsxPayload>,
) -> Result<Json<XlsxPreview>, ApiError> {
    let principal = authenticate(&state, &headers, false).await?;
    authorize_project(&state, project_id, principal.user_id, ProjectAction::Read).await?;
    let schema = current_schema(&state, project_id, schema_id).await?;
    let imported = import_workbook(&payload.content, &schema.definition, schema.revision_id)
        .map_err(xlsx_error)?;
    validate_import(&schema.definition, &imported.rows)?;
    let created = imported
        .rows
        .iter()
        .filter(|row| row.expected_version.is_none())
        .count();
    let updated = imported.rows.len() - created;
    Ok(Json(XlsxPreview {
        created,
        updated,
        rows: imported.rows,
    }))
}

async fn commit_xlsx(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, schema_id)): Path<(ProjectId, SchemaId)>,
    Json(payload): Json<XlsxPayload>,
) -> Result<Json<Vec<StoredRow>>, ApiError> {
    let principal = authenticate(&state, &headers, true).await?;
    authorize_project(&state, project_id, principal.user_id, ProjectAction::Write).await?;
    let schema = current_schema(&state, project_id, schema_id).await?;
    let imported = import_workbook(&payload.content, &schema.definition, schema.revision_id)
        .map_err(xlsx_error)?;
    validate_import(&schema.definition, &imported.rows)?;
    let writes = imported
        .rows
        .into_iter()
        .map(|imported| RowWrite {
            row: imported.row,
            expected_version: imported.expected_version,
        })
        .collect::<Vec<_>>();
    Ok(Json(
        save_rows_atomic(
            &state.pool,
            &writes,
            principal.user_id,
            project_id,
            "xlsx.imported",
        )
        .await?,
    ))
}

fn validate_import(
    schema: &SchemaDefinition,
    rows: &[datahub_xlsx::ImportedRow],
) -> Result<(), ApiError> {
    let issues = rows
        .iter()
        .flat_map(|imported| validate_row(schema, &imported.row))
        .collect::<Vec<_>>();
    if issues.is_empty() {
        Ok(())
    } else {
        Err(ApiError::validation(json!(issues)))
    }
}

async fn current_schema(
    state: &AppState,
    project_id: ProjectId,
    schema_id: SchemaId,
) -> Result<StoredSchema, ApiError> {
    list_schemas(&state.pool, project_id)
        .await?
        .into_iter()
        .find(|schema| schema.definition.id == schema_id)
        .ok_or_else(|| datahub_persistence_pg::RepositoryError::NotFound.into())
}

#[allow(clippy::needless_pass_by_value)]
fn formula_error(error: datahub_formula::FormulaError) -> ApiError {
    ApiError::validation(json!({"formula": error.to_string()}))
}

fn xlsx_error(error: XlsxError) -> ApiError {
    match error {
        XlsxError::ForeignSchema { .. } | XlsxError::StaleRevision { .. } => {
            ApiError::conflict(error.to_string())
        }
        XlsxError::Writer(_) | XlsxError::Reader(_) | XlsxError::Json(_) => {
            ApiError::bad_request("XLSX file could not be read")
        }
        other => ApiError::validation(json!({"xlsx": other.to_string()})),
    }
}

#[derive(Deserialize)]
struct CreateBuildRequest {
    target: CompilationTarget,
    #[serde(default)]
    audience: Audience,
}

async fn builds(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<ProjectId>,
) -> Result<Json<Vec<BuildRecord>>, ApiError> {
    let principal = authenticate(&state, &headers, false).await?;
    authorize_project(&state, project_id, principal.user_id, ProjectAction::Read).await?;
    Ok(Json(list_builds(&state.pool, project_id).await?))
}

async fn create_build_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<ProjectId>,
    Json(request): Json<CreateBuildRequest>,
) -> Result<Json<BuildRecord>, ApiError> {
    let principal = authenticate(&state, &headers, true).await?;
    authorize_project(&state, project_id, principal.user_id, ProjectAction::Write).await?;
    let schemas = list_schemas(&state.pool, project_id).await?;
    let mut artifacts = Vec::new();
    for schema in schemas.iter().filter(|schema| {
        schema.definition.target.includes(request.target)
            && schema.definition.target.includes_audience(request.audience)
    }) {
        let stored_rows = list_rows(&state.pool, schema.definition.id).await?;
        let rows = stored_rows
            .into_iter()
            .map(|stored| stored.row)
            .collect::<Vec<_>>();
        for artifact in [
            generate_code_for_audience(&schema.definition, request.target, request.audience),
            generate_json_for_audience(&schema.definition, &rows, request.target, request.audience),
            generate_csv_for_audience(&schema.definition, &rows, request.target, request.audience),
        ] {
            let artifact = artifact.map_err(export_error)?;
            artifacts.push(BuildArtifact {
                path: artifact.path,
                media_type: artifact.media_type,
                sha256: artifact.sha256,
                content: artifact.content,
            });
        }
    }
    let language = match request.target {
        CompilationTarget::Rust => "rust",
        CompilationTarget::CSharp => "c_sharp",
        CompilationTarget::TypeScript => "type_script",
    };
    let audience = match request.audience {
        Audience::Client => "client",
        Audience::Server => "server",
        Audience::Editor => "editor",
    };
    let target = format!("{audience}:{language}");
    Ok(Json(
        record_build(
            &state.pool,
            BuildId::new(),
            project_id,
            principal.user_id,
            &target,
            &artifacts,
        )
        .await?,
    ))
}

async fn sync_status_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<ProjectId>,
) -> Result<Json<SyncStatus>, ApiError> {
    let principal = authenticate(&state, &headers, false).await?;
    authorize_project(&state, project_id, principal.user_id, ProjectAction::Read).await?;
    Ok(Json(sync_status(&state.pool, project_id).await?))
}

fn export_error(error: ExportError) -> ApiError {
    match error {
        ExportError::Validation(issues) => ApiError::validation(json!(issues)),
        ExportError::Serialization(_) => ApiError::internal("artifact generation failed"),
    }
}

async fn authenticate(
    state: &AppState,
    headers: &HeaderMap,
    require_csrf: bool,
) -> Result<SessionPrincipal, ApiError> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty())
        .ok_or_else(ApiError::unauthorized)?;
    let principal = session_principal(&state.pool, &digest_token(token))
        .await?
        .ok_or_else(ApiError::unauthorized)?;
    if require_csrf {
        let csrf = headers
            .get("x-csrf-token")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| ApiError::forbidden("X-CSRF-Token is required for mutations"))?;
        if digest_token(csrf) != principal.csrf_digest {
            return Err(ApiError::forbidden("CSRF token is invalid"));
        }
    }
    Ok(principal)
}

async fn authorize_project(
    state: &AppState,
    project_id: ProjectId,
    user_id: UserId,
    action: ProjectAction,
) -> Result<ProjectRole, ApiError> {
    let role = project_role(&state.pool, project_id, user_id)
        .await?
        .ok_or_else(|| ApiError::forbidden("project membership is required"))?;
    if !role.allows(action) {
        return Err(ApiError::forbidden(
            "project role does not allow this operation",
        ));
    }
    Ok(role)
}

fn validate_username(username: &str) -> Result<(), ApiError> {
    let length = username.trim().chars().count();
    if !(3..=64).contains(&length)
        || !username
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        return Err(ApiError::bad_request(
            "username must be 3-64 ASCII letters, numbers, underscores, or hyphens",
        ));
    }
    Ok(())
}
