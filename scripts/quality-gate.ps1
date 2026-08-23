[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$qualityProject = 'datahub-quality'
$qualityRoot = Split-Path -Parent $PSScriptRoot

function Invoke-Checked {
    param(
        [Parameter(Mandatory)] [string] $Label,
        [Parameter(Mandatory)] [scriptblock] $Command
    )

    Write-Host "==> $Label"
    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "$Label failed with exit code $LASTEXITCODE"
    }
}

Push-Location $qualityRoot
try {
    Invoke-Checked 'Rust formatting' { cargo fmt --all -- --check }
    Invoke-Checked 'Rust linting' {
        cargo clippy --workspace --all-targets --all-features -- -D warnings
    }
    Invoke-Checked 'Rust tests' {
        cargo test --workspace --all-features -- --test-threads=2
    }

    Invoke-Checked 'Install web dependencies' { pnpm install --frozen-lockfile }
    Invoke-Checked 'Web linting' { pnpm web:lint }
    Invoke-Checked 'Web type checking' { pnpm web:typecheck }
    Invoke-Checked 'Web tests' { pnpm web:test }
    Invoke-Checked 'Web production build' { pnpm web:build }

    $env:POSTGRES_DB = 'datahub_quality'
    $env:POSTGRES_USER = 'datahub_quality'
    $env:POSTGRES_PASSWORD = 'datahub_quality_local_only'
    $env:POSTGRES_PORT = '15432'
    $env:DATAHUB_API_PORT = '18080'
    $env:DATAHUB_WEB_PORT = '13000'
    $env:DATAHUB_IMAGE_TAG = 'quality'

    Invoke-Checked 'Validate Docker Compose' {
        docker compose -p $qualityProject config --quiet
    }
    Invoke-Checked 'Build and start isolated Docker stack' {
        docker compose -p $qualityProject up --build --detach --wait --wait-timeout 300
    }

    Write-Host '==> HTTP smoke tests'
    $live = Invoke-RestMethod -Uri 'http://127.0.0.1:18080/health/live'
    $ready = Invoke-RestMethod -Uri 'http://127.0.0.1:18080/health/ready'
    $proxyReady = Invoke-RestMethod -Uri 'http://127.0.0.1:13000/api/health/ready'
    $web = Invoke-WebRequest -UseBasicParsing -Uri 'http://127.0.0.1:13000/'
    if ($live.status -ne 'ok' -or $ready.status -ne 'ok' -or
        $proxyReady.status -ne 'ok' -or $web.StatusCode -ne 200) {
        throw 'HTTP smoke tests returned an unexpected response'
    }

    Write-Host '==> Authentication and RBAC integration tests'
    $apiRoot = 'http://127.0.0.1:13000/api/v1'
    $setup = Invoke-RestMethod -Uri "$apiRoot/setup"
    if (-not $setup.requires_bootstrap) {
        throw 'Fresh quality database unexpectedly reports completed bootstrap'
    }
    $admin = Invoke-RestMethod -Method Post -Uri "$apiRoot/auth/bootstrap" `
        -ContentType 'application/json' `
        -Body (@{ username = 'quality-admin'; password = 'quality-admin-password' } | ConvertTo-Json)
    $adminHeaders = @{
        Authorization = "Bearer $($admin.token)"
        'X-CSRF-Token' = $admin.csrf_token
    }
    $viewer = Invoke-RestMethod -Method Post -Uri "$apiRoot/users" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ username = 'quality-viewer'; password = 'quality-viewer-password' } | ConvertTo-Json)
    $project = Invoke-RestMethod -Method Post -Uri "$apiRoot/projects" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ name = 'Quality Project'; description = 'Automated integration fixture' } | ConvertTo-Json)
    Invoke-RestMethod -Method Put `
        -Uri "$apiRoot/projects/$($project.id)/members/$($viewer.id)" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ role = 'viewer' } | ConvertTo-Json) | Out-Null

    $viewerAuth = Invoke-RestMethod -Method Post -Uri "$apiRoot/auth/login" `
        -ContentType 'application/json' `
        -Body (@{ username = 'quality-viewer'; password = 'quality-viewer-password' } | ConvertTo-Json)
    $viewerHeaders = @{
        Authorization = "Bearer $($viewerAuth.token)"
        'X-CSRF-Token' = $viewerAuth.csrf_token
    }
    $deniedStatus = 0
    try {
        Invoke-RestMethod -Method Put `
            -Uri "$apiRoot/projects/$($project.id)/members/$($viewer.id)" `
            -Headers $viewerHeaders -ContentType 'application/json' `
            -Body (@{ role = 'admin' } | ConvertTo-Json) | Out-Null
    }
    catch {
        $deniedStatus = [int]$_.Exception.Response.StatusCode
    }
    if ($deniedStatus -ne 403) {
        throw "Viewer privilege escalation returned HTTP $deniedStatus instead of 403"
    }

    Write-Host '==> Schema, row, validation, and optimistic-conflict integration tests'
    $schemaId = [guid]::NewGuid().ToString()
    $fieldId = [guid]::NewGuid().ToString()
    $serverFieldId = [guid]::NewGuid().ToString()
    $targetRule = @{
        include = @('rust', 'c_sharp', 'type_script')
        audiences = @('client', 'server')
        rename = @{}
    }
    $schemaDefinition = @{
        id = $schemaId
        project_id = $project.id
        name = 'Monster'
        description = 'Quality fixture schema'
        fields = @(
            @{
                id = $fieldId
                name = 'level'
                description = 'Monster level'
                ty = @{ kind = 'integer'; min = 1; max = 100 }
                default = $null
                target = $targetRule
            },
            @{
                id = $serverFieldId
                name = 'server_secret'
                description = 'Server-only field'
                ty = @{ kind = 'integer'; min = 0; max = 9999 }
                default = $null
                target = @{
                    include = @('rust', 'c_sharp', 'type_script')
                    audiences = @('server')
                    rename = @{}
                }
            }
        )
        target = $targetRule
    }
    $schema = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/schemas" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ definition = $schemaDefinition; expected_version = $null } | ConvertTo-Json -Depth 20)
    if ($schema.version -ne 1) { throw 'New schema did not start at version 1' }

    $schemaDefinition.description = 'Updated fixture schema'
    $updatedSchema = Invoke-RestMethod -Method Put `
        -Uri "$apiRoot/projects/$($project.id)/schemas/$schemaId" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ definition = $schemaDefinition; expected_version = 1 } | ConvertTo-Json -Depth 20)
    if ($updatedSchema.version -ne 2) { throw 'Schema update did not advance to version 2' }

    $conflictStatus = 0
    try {
        Invoke-RestMethod -Method Put `
            -Uri "$apiRoot/projects/$($project.id)/schemas/$schemaId" `
            -Headers $adminHeaders -ContentType 'application/json' `
            -Body (@{ definition = $schemaDefinition; expected_version = 1 } | ConvertTo-Json -Depth 20) | Out-Null
    }
    catch { $conflictStatus = [int]$_.Exception.Response.StatusCode }
    if ($conflictStatus -ne 409) {
        throw "Stale schema update returned HTTP $conflictStatus instead of 409"
    }

    $rowValues = @{}
    $rowValues[$fieldId] = @{ kind = 'integer'; value = 50 }
    $rowValues[$serverFieldId] = @{ kind = 'integer'; value = 9001 }
    $rowId = [guid]::NewGuid().ToString()
    $row = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/schemas/$schemaId/rows" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{
            row = @{
                id = $rowId
                schema_id = $schemaId
                revision_id = [guid]::NewGuid().ToString()
                values = $rowValues
            }
            expected_version = $null
        } | ConvertTo-Json -Depth 20)
    if ($row.version -ne 1) { throw 'New row did not start at version 1' }

    $tableView = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/schemas/$schemaId/views" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ block_size = 512; sort = @(); filters = @() } | ConvertTo-Json -Depth 10)
    $firstBlock = Invoke-RestMethod `
        -Uri "$apiRoot/table-views/$($tableView.view_id)/blocks/0" -Headers $adminHeaders
    if ($tableView.total_rows -ne 1 -or $firstBlock.rows.Count -ne 1 -or
        -not $tableView.data_revision) {
        throw 'Block table view did not return the committed data revision and row'
    }

    $invalidValues = @{}
    $invalidValues[$fieldId] = @{ kind = 'integer'; value = 101 }
    $validationStatus = 0
    try {
        Invoke-RestMethod -Method Post `
            -Uri "$apiRoot/projects/$($project.id)/schemas/$schemaId/rows" `
            -Headers $adminHeaders -ContentType 'application/json' `
            -Body (@{
                row = @{
                    id = [guid]::NewGuid().ToString()
                    schema_id = $schemaId
                    revision_id = [guid]::NewGuid().ToString()
                    values = $invalidValues
                }
                expected_version = $null
            } | ConvertTo-Json -Depth 20) | Out-Null
    }
    catch { $validationStatus = [int]$_.Exception.Response.StatusCode }
    if ($validationStatus -ne 422) {
        throw "Invalid row returned HTTP $validationStatus instead of 422"
    }

    Write-Host '==> Deterministic build and PostgreSQL outbox projection tests'
    foreach ($target in @('rust', 'c_sharp', 'type_script')) {
        $build = Invoke-RestMethod -Method Post `
            -Uri "$apiRoot/projects/$($project.id)/builds" `
            -Headers $adminHeaders -ContentType 'application/json' `
            -Body (@{ target = $target } | ConvertTo-Json)
        if ($build.status -ne 'succeeded' -or $build.artifacts.Count -ne 3) {
            throw "Build for $target did not produce code, JSON, and CSV artifacts"
        }
        foreach ($artifact in $build.artifacts) {
            if ($artifact.sha256 -notmatch '^[0-9a-f]{64}$' -or $artifact.content.Count -eq 0) {
                throw "Build artifact $($artifact.path) is empty or missing its SHA-256 hash"
            }
        }
        $clientJson = $build.artifacts | Where-Object { $_.path -like '*.json' } | Select-Object -First 1
        $clientJsonText = [Text.Encoding]::UTF8.GetString([byte[]]$clientJson.content)
        if ($clientJsonText -match 'serverSecret') {
            throw "Client $target build leaked the server-only field"
        }
    }

    $serverBuild = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/builds" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ target = 'rust'; audience = 'server' } | ConvertTo-Json)
    $serverJson = $serverBuild.artifacts | Where-Object { $_.path -like '*.json' } | Select-Object -First 1
    $serverJsonText = [Text.Encoding]::UTF8.GetString([byte[]]$serverJson.content)
    if ($serverJsonText -notmatch 'serverSecret') {
        throw 'Server build omitted its server-only field'
    }

    $sync = $null
    for ($attempt = 0; $attempt -lt 15; $attempt++) {
        $sync = Invoke-RestMethod -Uri "$apiRoot/projects/$($project.id)/sync-status" `
            -Headers $adminHeaders
        if ($sync.pending -eq 0 -and $sync.projected_schemas -eq 1 -and
            $sync.projected_rows -eq 1) {
            break
        }
        Start-Sleep -Seconds 1
    }
    if ($sync.pending -ne 0 -or $sync.failed -ne 0 -or
        $sync.projected_schemas -ne 1 -or $sync.projected_rows -ne 1) {
        throw "Outbox projection did not converge: $($sync | ConvertTo-Json -Compress)"
    }

    Write-Host '==> Reference existence validation test'
    $referenceSchemaId = [guid]::NewGuid().ToString()
    $referenceFieldId = [guid]::NewGuid().ToString()
    $referenceDefinition = @{
        id = $referenceSchemaId
        project_id = $project.id
        name = 'DropTable'
        description = 'Reference validation fixture'
        fields = @(@{
            id = $referenceFieldId
            name = 'item'
            description = ''
            ty = @{ kind = 'reference'; schema_id = $schemaId; mode = 'hard' }
            default = $null
            target = $targetRule
        })
        target = $targetRule
    }
    Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/schemas" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ definition = $referenceDefinition } | ConvertTo-Json -Depth 20) | Out-Null
    $invalidReferenceValues = @{}
    $invalidReferenceValues[$referenceFieldId] = @{
        kind = 'reference'
        value = @{ schema_id = $schemaId; row_id = [guid]::NewGuid().ToString() }
    }
    $referenceStatus = 0
    try {
        Invoke-RestMethod -Method Post `
            -Uri "$apiRoot/projects/$($project.id)/schemas/$referenceSchemaId/rows" `
            -Headers $adminHeaders -ContentType 'application/json' `
            -Body (@{ row = @{
                id = [guid]::NewGuid().ToString()
                schema_id = $referenceSchemaId
                revision_id = [guid]::NewGuid().ToString()
                values = $invalidReferenceValues
            }} | ConvertTo-Json -Depth 20) | Out-Null
    }
    catch { $referenceStatus = [int]$_.Exception.Response.StatusCode }
    if ($referenceStatus -ne 422) {
        throw "Missing referenced row returned HTTP $referenceStatus instead of 422"
    }

    Invoke-Checked 'Verify SQLx migration' {
        docker compose -p $qualityProject exec -T postgres psql `
            -U datahub_quality -d datahub_quality -v ON_ERROR_STOP=1 `
            -c 'SELECT version, description, success FROM _sqlx_migrations ORDER BY version; SELECT COUNT(*) AS schema_revisions FROM datahub_schema_revisions; SELECT COUNT(*) AS row_revisions FROM datahub_row_revisions; SELECT COUNT(*) AS data_revisions FROM datahub_data_revisions; SELECT COUNT(*) AS builds FROM datahub_jobs WHERE kind = ''build''; SELECT COUNT(*) AS artifacts FROM datahub_build_artifacts; SELECT COUNT(*) AS projected_schemas FROM datahub_projection_schemas; SELECT COUNT(*) AS projected_rows FROM datahub_projection_rows; SELECT COUNT(*) AS audit_events FROM datahub_audit_events; SELECT COUNT(*) AS outbox_events FROM datahub_outbox_events;'
    }

    Invoke-Checked 'Insert persistence marker' {
        docker compose -p $qualityProject exec -T postgres psql `
            -U datahub_quality -d datahub_quality -v ON_ERROR_STOP=1 `
            -c "INSERT INTO datahub_system_info (key, value) VALUES ('quality-persistence', jsonb_build_object('verified', true)) ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value;"
    }
    Invoke-Checked 'Restart stack without deleting its volume' {
        docker compose -p $qualityProject down
        if ($LASTEXITCODE -ne 0) { return }
        docker compose -p $qualityProject up --detach --wait --wait-timeout 120
    }

    $persisted = docker compose -p $qualityProject exec -T postgres psql `
        -U datahub_quality -d datahub_quality -v ON_ERROR_STOP=1 -Atc `
        "SELECT value->>'verified' FROM datahub_system_info WHERE key = 'quality-persistence';"
    if ($LASTEXITCODE -ne 0 -or $persisted.Trim() -ne 'true') {
        throw "PostgreSQL persistence check failed: $persisted"
    }

    Write-Host 'DataHub local quality gate passed.' -ForegroundColor Green
}
finally {
    docker compose -p $qualityProject down --volumes --remove-orphans
    Pop-Location
}
