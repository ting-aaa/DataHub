[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$qualityProject = 'datahub-quality'
$qualityRoot = Split-Path -Parent $PSScriptRoot
$pluginTestRoot = $null
$backupRoot = $null

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
    Invoke-Checked 'Tracked-file secret scan' {
        pwsh -NoProfile -File scripts/secret-scan.ps1
    }

    Write-Host '==> Wasmtime Component/WIT plugin sandbox tests'
    Invoke-Checked 'Install Rust WebAssembly target' {
        rustup target add wasm32-unknown-unknown --toolchain 1.96.0
    }
    Invoke-Checked 'Compile example WIT plugin' {
        cargo build --manifest-path examples/datahub-echo-plugin/Cargo.toml `
            --target wasm32-unknown-unknown --release
    }
    $pluginTestRoot = [IO.Path]::GetFullPath((Join-Path ([IO.Path]::GetTempPath()) "datahub-plugin-$([guid]::NewGuid())"))
    $pluginTempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
    if (-not $pluginTestRoot.StartsWith($pluginTempRoot, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Plugin test directory escaped the system temp root: $pluginTestRoot"
    }
    New-Item -ItemType Directory -Path $pluginTestRoot | Out-Null
    $corePlugin = Join-Path $qualityRoot 'examples/datahub-echo-plugin/target/wasm32-unknown-unknown/release/datahub_echo_plugin.wasm'
    $componentPlugin = Join-Path $pluginTestRoot 'plugin.wasm'
    Invoke-Checked 'Encode example as a WebAssembly Component' {
        cargo run --quiet -p datahub-plugin-host --example componentize -- $corePlugin $componentPlugin
    }
    $pluginHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $componentPlugin).Hash.ToLowerInvariant()
    $pluginManifest = @"
id = "echo-plugin"
version = "1.0.0"
api_version = "1.0.0"
component = "plugin.wasm"
sha256 = "$pluginHash"
output_file = "echo.json"

[capabilities]
read_inputs = ["input/data.bin"]
write_output_directory = "generated/echo"

[limits]
fuel = 10000000
memory_bytes = 67108864
timeout_ms = 500
max_input_bytes = 1048576
max_output_bytes = 1048576
"@
    Set-Content -LiteralPath (Join-Path $pluginTestRoot 'plugin.toml') -Value $pluginManifest -Encoding utf8
    $pluginSmoke = cargo run --quiet -p datahub-plugin-host -- run-package $pluginTestRoot hello
    if ($LASTEXITCODE -ne 0 -or $pluginSmoke -notmatch '^generated/echo/echo\.json ') {
        throw "Example Component plugin failed: $pluginSmoke"
    }
    cargo run --quiet -p datahub-plugin-host -- run-package $pluginTestRoot oversize 2>&1 | Out-Host
    if ($LASTEXITCODE -eq 0) { throw 'Plugin output quota did not stop oversized output' }
    cargo run --quiet -p datahub-plugin-host -- run-package $pluginTestRoot memory 2>&1 | Out-Host
    if ($LASTEXITCODE -eq 0) { throw 'Plugin memory quota did not stop excessive allocation' }
    cargo run --quiet -p datahub-plugin-host -- run-package $pluginTestRoot spin 2>&1 | Out-Host
    if ($LASTEXITCODE -eq 0) { throw 'Plugin fuel quota did not stop an infinite guest' }
    $timeoutManifest = $pluginManifest -replace 'fuel = 10000000', 'fuel = 9000000000000000000'
    $timeoutManifest = $timeoutManifest -replace 'timeout_ms = 500', 'timeout_ms = 10'
    Set-Content -LiteralPath (Join-Path $pluginTestRoot 'plugin.toml') -Value $timeoutManifest -Encoding utf8
    cargo run --quiet -p datahub-plugin-host -- run-package $pluginTestRoot spin 2>&1 | Out-Host
    if ($LASTEXITCODE -eq 0) { throw 'Plugin wall-clock timeout did not stop an infinite guest' }

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
    $env:DATAHUB_AUTH_RATE_LIMIT = '3'
    $env:DATAHUB_MUTATION_RATE_LIMIT = '5000'
    $env:DATAHUB_RATE_LIMIT_WINDOW_SECONDS = '2'

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
    $metrics = Invoke-WebRequest -UseBasicParsing -Uri 'http://127.0.0.1:18080/metrics'
    if ($metrics.StatusCode -ne 200 -or $metrics.Content -notmatch 'datahub_database_ready 1' -or
        $metrics.Headers['X-Request-ID'] -notmatch '^[0-9a-f-]{36}$') {
        throw 'API metrics or request correlation header is unavailable'
    }
    $pluginMetrics = docker compose -p $qualityProject exec -T plugin-host `
        curl --fail --silent http://127.0.0.1:8081/metrics
    $pluginMetricsText = $pluginMetrics -join "`n"
    if ($LASTEXITCODE -ne 0 -or $pluginMetricsText -notmatch 'datahub_plugin_quota_rejections_total') {
        throw 'Plugin metrics endpoint is unavailable'
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

    Write-Host '==> Authentication rate-limit and recovery-window tests'
    for ($rateAttempt = 1; $rateAttempt -le 3; $rateAttempt++) {
        $unauthorizedStatus = 0
        try {
            Invoke-RestMethod -Method Post -Uri "$apiRoot/auth/login" `
                -ContentType 'application/json' `
                -Body (@{ username = 'quality-rate-probe'; password = 'definitely-wrong-password' } | ConvertTo-Json) | Out-Null
        }
        catch { $unauthorizedStatus = [int]$_.Exception.Response.StatusCode }
        if ($unauthorizedStatus -ne 401) {
            throw "Rate-limit warm-up attempt $rateAttempt returned HTTP $unauthorizedStatus"
        }
    }
    $limitedStatus = 0
    try {
        Invoke-RestMethod -Method Post -Uri "$apiRoot/auth/login" `
            -Headers @{ 'X-Forwarded-For' = '203.0.113.77' } `
            -ContentType 'application/json' `
            -Body (@{ username = 'quality-rate-probe'; password = 'definitely-wrong-password' } | ConvertTo-Json) | Out-Null
    }
    catch { $limitedStatus = [int]$_.Exception.Response.StatusCode }
    if ($limitedStatus -ne 429) {
        throw "Authentication rate limit returned HTTP $limitedStatus instead of 429"
    }
    Start-Sleep -Seconds 3
    $recoveredStatus = 0
    try {
        Invoke-RestMethod -Method Post -Uri "$apiRoot/auth/login" `
            -ContentType 'application/json' `
            -Body (@{ username = 'quality-rate-probe'; password = 'definitely-wrong-password' } | ConvertTo-Json) | Out-Null
    }
    catch { $recoveredStatus = [int]$_.Exception.Response.StatusCode }
    if ($recoveredStatus -ne 401) {
        throw "Authentication rate limit did not recover after its window: HTTP $recoveredStatus"
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

    Write-Host '==> FieldId formula Native/WebAssembly parity and atomic apply tests'
    $formulaSet = Invoke-RestMethod -Method Put `
        -Uri "$apiRoot/projects/$($project.id)/schemas/$schemaId/formulas" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{
            definitions = @(@{ field_id = $serverFieldId; source = 'level * 2' })
            expected_version = $null
        } | ConvertTo-Json -Depth 10)
    if ($formulaSet.version -ne 1 -or $formulaSet.schema_revision_id -ne $updatedSchema.revision_id) {
        throw 'Formula set was not stored against the current schema revision'
    }
    $nativeFormulaPreview = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/schemas/$schemaId/formulas/preview" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ runtime = 'native' } | ConvertTo-Json)
    $wasmFormulaPreview = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/schemas/$schemaId/formulas/preview" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ runtime = 'wasm' } | ConvertTo-Json)
    $nativeFormulaJson = $nativeFormulaPreview | ConvertTo-Json -Depth 20 -Compress
    $wasmFormulaJson = $wasmFormulaPreview | ConvertTo-Json -Depth 20 -Compress
    if ($nativeFormulaJson -ne $wasmFormulaJson -or $nativeFormulaPreview.Count -ne 1) {
        throw 'Native and WebAssembly formula previews differ'
    }
    $computedValue = $nativeFormulaPreview[0].after.values.PSObject.Properties[$serverFieldId].Value.value
    if ($computedValue -ne 100) { throw "Formula preview returned $computedValue instead of 100" }
    $formulaApplied = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/schemas/$schemaId/formulas/apply" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ runtime = 'wasm' } | ConvertTo-Json)
    if ($formulaApplied.Count -ne 1 -or $formulaApplied[0].version -ne 2) {
        throw 'Formula apply did not atomically advance the changed row'
    }

    Write-Host '==> XLSX stable-ID preview and optimistic rollback tests'
    $staleRowId = 'ffffffff-ffff-7fff-bfff-ffffffffffff'
    $staleValues = @{}
    $staleValues[$fieldId] = @{ kind = 'integer'; value = 10 }
    $staleValues[$serverFieldId] = @{ kind = 'integer'; value = 0 }
    $staleRow = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/schemas/$schemaId/rows" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{
            row = @{
                id = $staleRowId
                schema_id = $schemaId
                revision_id = [guid]::NewGuid().ToString()
                values = $staleValues
            }
            expected_version = $null
        } | ConvertTo-Json -Depth 20)
    $xlsxArtifact = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/schemas/$schemaId/xlsx/export" `
        -Headers $adminHeaders -ContentType 'application/json'
    if ($xlsxArtifact.content.Count -lt 1000 -or $xlsxArtifact.file_name -notlike '*.xlsx') {
        throw 'XLSX export did not return a workbook artifact'
    }
    $xlsxPayload = @{ content = @($xlsxArtifact.content) } | ConvertTo-Json -Depth 10 -Compress
    $xlsxPreview = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/schemas/$schemaId/xlsx/preview" `
        -Headers $adminHeaders -ContentType 'application/json' -Body $xlsxPayload
    if ($xlsxPreview.created -ne 0 -or $xlsxPreview.updated -ne 2) {
        throw 'XLSX preview did not preserve both stable row identities'
    }
    $staleValues[$serverFieldId] = @{ kind = 'integer'; value = 77 }
    $staleRowUpdated = Invoke-RestMethod -Method Put `
        -Uri "$apiRoot/projects/$($project.id)/schemas/$schemaId/rows/$staleRowId" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{
            row = @{
                id = $staleRowId
                schema_id = $schemaId
                revision_id = $staleRow.row.revision_id
                values = $staleValues
            }
            expected_version = 1
        } | ConvertTo-Json -Depth 20)
    $rowsBeforeRollback = Invoke-RestMethod `
        -Uri "$apiRoot/projects/$($project.id)/schemas/$schemaId/rows" -Headers $adminHeaders
    $firstBeforeRollback = $rowsBeforeRollback | Where-Object { $_.row.id -eq $rowId }
    $xlsxConflictStatus = 0
    try {
        Invoke-RestMethod -Method Post `
            -Uri "$apiRoot/projects/$($project.id)/schemas/$schemaId/xlsx/commit" `
            -Headers $adminHeaders -ContentType 'application/json' -Body $xlsxPayload | Out-Null
    }
    catch { $xlsxConflictStatus = [int]$_.Exception.Response.StatusCode }
    if ($xlsxConflictStatus -ne 409) {
        throw "Stale XLSX commit returned HTTP $xlsxConflictStatus instead of 409"
    }
    $rowsAfterRollback = Invoke-RestMethod `
        -Uri "$apiRoot/projects/$($project.id)/schemas/$schemaId/rows" -Headers $adminHeaders
    $firstAfterRollback = $rowsAfterRollback | Where-Object { $_.row.id -eq $rowId }
    $staleAfterRollback = $rowsAfterRollback | Where-Object { $_.row.id -eq $staleRowId }
    if ($firstAfterRollback.version -ne $firstBeforeRollback.version -or
        $staleAfterRollback.version -ne $staleRowUpdated.version) {
        throw 'XLSX version conflict did not roll back the full transaction'
    }

    $tableView = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/schemas/$schemaId/views" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ block_size = 512; sort = @(); filters = @() } | ConvertTo-Json -Depth 10)
    $firstBlock = Invoke-RestMethod `
        -Uri "$apiRoot/table-views/$($tableView.view_id)/blocks/0" -Headers $adminHeaders
    if ($tableView.total_rows -ne 2 -or $firstBlock.rows.Count -ne 2 -or
        -not $tableView.data_revision) {
        throw 'Block table view did not return the committed data revision and row'
    }
    $secondBlock = Invoke-RestMethod `
        -Uri "$apiRoot/table-views/$($tableView.view_id)/blocks/1" -Headers $adminHeaders
    if ($secondBlock.rows.Count -ne 0) {
        throw 'Table view returned rows beyond its final block'
    }

    $filteredView = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/schemas/$schemaId/views" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{
            block_size = 256
            filters = @(@{ field_id = $fieldId; value = @{ kind = 'integer'; value = 50 } })
            sort = @(@{ field_id = $fieldId; direction = 'desc' })
        } | ConvertTo-Json -Depth 10)
    $filteredBlock = Invoke-RestMethod `
        -Uri "$apiRoot/table-views/$($filteredView.view_id)/blocks/0" -Headers $adminHeaders
    if ($filteredView.total_rows -ne 1 -or $filteredBlock.rows.Count -ne 1) {
        throw 'Server-side table view filtering or sorting did not return the matching row'
    }

    $missingView = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/schemas/$schemaId/views" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{
            block_size = 256
            filters = @(@{ field_id = $fieldId; value = @{ kind = 'integer'; value = 51 } })
            sort = @()
        } | ConvertTo-Json -Depth 10)
    if ($missingView.total_rows -ne 0) {
        throw 'Server-side table view filtering returned a non-matching row'
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

    Write-Host '==> Deterministic build, codec, generated-code, and PostgreSQL outbox tests'
    $compileRoot = [IO.Path]::GetFullPath((Join-Path ([IO.Path]::GetTempPath()) "datahub-generated-$([guid]::NewGuid())"))
    $systemTempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
    if (-not $compileRoot.StartsWith($systemTempRoot, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Generated-code test directory escaped the system temp root: $compileRoot"
    }
    New-Item -ItemType Directory -Path $compileRoot | Out-Null
    $firstRustBuild = $null
    foreach ($target in @('rust', 'c_sharp', 'type_script')) {
        $build = Invoke-RestMethod -Method Post `
            -Uri "$apiRoot/projects/$($project.id)/builds" `
            -Headers $adminHeaders -ContentType 'application/json' `
            -Body (@{ target = $target } | ConvertTo-Json)
        if ($build.status -ne 'succeeded' -or $build.artifacts.Count -ne 9) {
            throw "Build for $target did not produce code, six data codecs, Protobuf schema, and manifest"
        }
        if ($build.input_hash -notmatch '^[0-9a-f]{64}$' -or $null -eq $build.manifest) {
            throw "Build for $target did not persist its input hash and manifest"
        }
        foreach ($artifact in $build.artifacts) {
            if ($artifact.sha256 -notmatch '^[0-9a-f]{64}$' -or $artifact.content.Count -eq 0) {
                throw "Build artifact $($artifact.path) is empty or missing its SHA-256 hash"
            }
        }
        $manifestArtifact = $build.artifacts | Where-Object { $_.path -eq 'manifest.json' } | Select-Object -First 1
        if ($manifestArtifact.sha256 -ne $build.input_hash -or $build.manifest.format -ne 'datahub-build-v1' -or
            $build.manifest.artifacts.Count -ne 8) {
            throw "Build for $target returned an inconsistent deterministic manifest"
        }
        $requiredExtensions = @('.json', '.csv', '.xml', '.bson', '.proto', '.pb', '.lua')
        foreach ($extension in $requiredExtensions) {
            if (($build.artifacts | Where-Object {
                $_.path.StartsWith('data/', [StringComparison]::Ordinal) -and $_.path.EndsWith($extension)
            }).Count -ne 1) {
                throw "Build for $target did not emit exactly one $extension artifact"
            }
        }
        $clientJson = $build.artifacts | Where-Object { $_.path -like 'data/*.json' } | Select-Object -First 1
        $clientJsonText = [Text.Encoding]::UTF8.GetString([byte[]]$clientJson.content)
        if ($clientJsonText -match 'serverSecret') {
            throw "Client $target build leaked the server-only field"
        }

        $source = $build.artifacts | Where-Object { $_.path -like 'code/*' } | Select-Object -First 1
        switch ($target) {
            'rust' {
                $firstRustBuild = $build
                $rustRoot = Join-Path $compileRoot 'rust'
                New-Item -ItemType Directory -Path (Join-Path $rustRoot 'src') | Out-Null
                [IO.File]::WriteAllBytes((Join-Path $rustRoot 'src/lib.rs'), [byte[]]$source.content)
                @'
[package]
name = "datahub-generated-check"
version = "0.0.0"
edition = "2024"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
'@ | Set-Content -LiteralPath (Join-Path $rustRoot 'Cargo.toml') -Encoding utf8
                Invoke-Checked 'Compile generated Rust' {
                    cargo check --quiet --manifest-path (Join-Path $rustRoot 'Cargo.toml')
                }
            }
            'c_sharp' {
                $csharpRoot = Join-Path $compileRoot 'csharp'
                New-Item -ItemType Directory -Path $csharpRoot | Out-Null
                [IO.File]::WriteAllBytes((Join-Path $csharpRoot 'Generated.cs'), [byte[]]$source.content)
                @'
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net9.0</TargetFramework>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
    <TreatWarningsAsErrors>true</TreatWarningsAsErrors>
  </PropertyGroup>
</Project>
'@ | Set-Content -LiteralPath (Join-Path $csharpRoot 'Generated.csproj') -Encoding utf8
                Invoke-Checked 'Compile generated C#' {
                    dotnet build (Join-Path $csharpRoot 'Generated.csproj') --nologo --verbosity quiet
                }
            }
            'type_script' {
                $typescriptPath = Join-Path $compileRoot 'generated.ts'
                [IO.File]::WriteAllBytes($typescriptPath, [byte[]]$source.content)
                Invoke-Checked 'Compile generated TypeScript' {
                    pnpm --dir web exec tsc $typescriptPath --noEmit --strict --target ES2022 --module ESNext --skipLibCheck
                }
            }
        }
    }

    $secondRustBuild = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/builds" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ target = 'rust' } | ConvertTo-Json)
    if ($secondRustBuild.input_hash -ne $firstRustBuild.input_hash) {
        throw 'Identical build inputs produced different manifest hashes'
    }
    $firstDigests = @($firstRustBuild.artifacts | Sort-Object path | ForEach-Object { "$($_.path):$($_.sha256)" })
    $secondDigests = @($secondRustBuild.artifacts | Sort-Object path | ForEach-Object { "$($_.path):$($_.sha256)" })
    if (Compare-Object -ReferenceObject $firstDigests -DifferenceObject $secondDigests) {
        throw 'Identical build inputs produced different artifact hashes'
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
            $sync.projected_rows -eq 2) {
            break
        }
        Start-Sleep -Seconds 1
    }
    if ($sync.pending -ne 0 -or $sync.retrying -ne 0 -or $sync.dead_lettered -ne 0 -or
        $sync.projected_schemas -ne 1 -or $sync.projected_rows -ne 2) {
        throw "Outbox projection did not converge: $($sync | ConvertTo-Json -Compress)"
    }
    $currentProjectedRows = docker compose -p $qualityProject exec -T postgres psql `
        -U datahub_quality -d datahub_quality -v ON_ERROR_STOP=1 -Atc `
        'SELECT COUNT(*) FROM datahub_projection_rows p JOIN datahub_config_rows r ON r.id = p.row_id WHERE p.source_version = r.version;'
    if ($LASTEXITCODE -ne 0 -or [int]$currentProjectedRows.Trim() -ne 2) {
        throw "Formula/XLSX row events did not update the current projection: $currentProjectedRows"
    }

    Write-Host '==> Projection DDL approval, retry/dead-letter, checkpoint, resync, release, and rollback tests'
    $initialPlan = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/projection-plans" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ schema_id = $schemaId } | ConvertTo-Json)
    if ($initialPlan.destructive -or $initialPlan.operations.Count -ne 1) {
        throw 'Initial PostgreSQL DDL plan was not a compatible CREATE TABLE operation'
    }
    $appliedInitialPlan = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/projection-plans/$($initialPlan.id)/apply" `
        -Headers $adminHeaders -ContentType 'application/json'
    if ($appliedInitialPlan.status -ne 'applied') { throw 'Compatible DDL plan was not applied' }

    $schemaDefinition.fields = @($schemaDefinition.fields | Where-Object { $_.id -ne $serverFieldId })
    $destructiveSchema = Invoke-RestMethod -Method Put `
        -Uri "$apiRoot/projects/$($project.id)/schemas/$schemaId" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ definition = $schemaDefinition; expected_version = 2 } | ConvertTo-Json -Depth 20)
    $destructivePlan = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/projection-plans" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ schema_id = $schemaId } | ConvertTo-Json)
    if (-not $destructivePlan.destructive) { throw 'Removed field did not produce destructive DDL' }
    $unapprovedPlanStatus = 0
    try {
        Invoke-RestMethod -Method Post `
            -Uri "$apiRoot/projects/$($project.id)/projection-plans/$($destructivePlan.id)/apply" `
            -Headers $adminHeaders -ContentType 'application/json' | Out-Null
    }
    catch { $unapprovedPlanStatus = [int]$_.Exception.Response.StatusCode }
    if ($unapprovedPlanStatus -ne 409) {
        throw "Unapproved destructive DDL returned HTTP $unapprovedPlanStatus instead of 409"
    }
    Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/projection-plans/$($destructivePlan.id)/approve" `
        -Headers $adminHeaders -ContentType 'application/json' | Out-Null
    $appliedDestructivePlan = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/projection-plans/$($destructivePlan.id)/apply" `
        -Headers $adminHeaders -ContentType 'application/json'
    if ($appliedDestructivePlan.status -ne 'applied') { throw 'Approved destructive DDL was not applied' }

    $poisonId = [guid]::NewGuid().ToString()
    $missingAggregateId = [guid]::NewGuid().ToString()
    $poisonKey = "quality-poison-$poisonId"
    Invoke-Checked 'Insert retryable poison outbox event' {
        docker compose -p $qualityProject exec -T postgres psql `
            -U datahub_quality -d datahub_quality -v ON_ERROR_STOP=1 `
            -c "INSERT INTO datahub_outbox_events (id, project_id, event_type, aggregate_type, aggregate_id, payload, idempotency_key) VALUES ('$poisonId', '$($project.id)', 'schema.saved', 'schema', '$missingAggregateId', '{}'::jsonb, '$poisonKey');"
    }
    for ($retry = 0; $retry -lt 6; $retry++) {
        docker compose -p $qualityProject exec -T postgres psql `
            -U datahub_quality -d datahub_quality -v ON_ERROR_STOP=1 -c `
            "UPDATE datahub_outbox_events SET available_at = NOW() WHERE idempotency_key = '$poisonKey' AND dead_lettered_at IS NULL;" | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'Failed to accelerate poison event retry' }
        Start-Sleep -Seconds 2
    }
    $poisonState = docker compose -p $qualityProject exec -T postgres psql `
        -U datahub_quality -d datahub_quality -v ON_ERROR_STOP=1 -Atc `
        "SELECT attempts || ':' || (dead_lettered_at IS NOT NULL)::text FROM datahub_outbox_events WHERE idempotency_key = '$poisonKey';"
    if ($LASTEXITCODE -ne 0 -or $poisonState.Trim() -ne '5:true') {
        throw "Poison event did not dead-letter after five isolated attempts: $poisonState"
    }

    Invoke-Checked 'Remove projection row before recovery test' {
        docker compose -p $qualityProject exec -T postgres psql `
            -U datahub_quality -d datahub_quality -v ON_ERROR_STOP=1 `
            -c "DELETE FROM datahub_projection_rows WHERE project_id = '$($project.id)';"
    }
    $resynced = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/sync/resync" `
        -Headers $adminHeaders -ContentType 'application/json'
    if ($resynced.projected_rows -ne 2 -or $resynced.checkpoint.status -ne 'ready' -or
        $resynced.dead_lettered -ne 1) {
        throw "Full resync did not restore projections/checkpoint while retaining dead letter: $($resynced | ConvertTo-Json -Compress)"
    }

    $environment = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/environments" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ name = 'production'; requires_approval = $true } | ConvertTo-Json)
    $releaseOne = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/releases" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ environment_id = $environment.id; build_id = $firstRustBuild.id; version = '1.0.0' } | ConvertTo-Json)
    $unapprovedReleaseStatus = 0
    try {
        Invoke-RestMethod -Method Post `
            -Uri "$apiRoot/projects/$($project.id)/releases/$($releaseOne.id)/publish" `
            -Headers $adminHeaders -ContentType 'application/json' | Out-Null
    }
    catch { $unapprovedReleaseStatus = [int]$_.Exception.Response.StatusCode }
    if ($unapprovedReleaseStatus -ne 409) {
        throw "Unapproved production release returned HTTP $unapprovedReleaseStatus instead of 409"
    }
    Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/releases/$($releaseOne.id)/approve" `
        -Headers $adminHeaders -ContentType 'application/json' | Out-Null
    $publishedOne = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/releases/$($releaseOne.id)/publish" `
        -Headers $adminHeaders -ContentType 'application/json'
    if ($publishedOne.status -ne 'published' -or $publishedOne.input_hash -ne $firstRustBuild.input_hash) {
        throw 'Approved release did not preserve and publish its deterministic build snapshot'
    }
    $releaseTwo = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/releases" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ environment_id = $environment.id; build_id = $secondRustBuild.id; version = '1.1.0' } | ConvertTo-Json)
    Invoke-RestMethod -Method Post -Uri "$apiRoot/projects/$($project.id)/releases/$($releaseTwo.id)/approve" -Headers $adminHeaders -ContentType 'application/json' | Out-Null
    Invoke-RestMethod -Method Post -Uri "$apiRoot/projects/$($project.id)/releases/$($releaseTwo.id)/publish" -Headers $adminHeaders -ContentType 'application/json' | Out-Null
    $rollback = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/environments/$($environment.id)/rollback" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ target_release_id = $releaseOne.id; version = 'rollback-1.0.0' } | ConvertTo-Json)
    $environments = Invoke-RestMethod -Uri "$apiRoot/projects/$($project.id)/environments" -Headers $adminHeaders
    $production = $environments | Where-Object { $_.id -eq $environment.id }
    $releaseHistory = Invoke-RestMethod -Uri "$apiRoot/projects/$($project.id)/releases" -Headers $adminHeaders
    $historicalOne = $releaseHistory | Where-Object { $_.id -eq $releaseOne.id }
    if ($rollback.status -ne 'published' -or $rollback.rollback_of -ne $releaseOne.id -or
        $rollback.input_hash -ne $releaseOne.input_hash -or $production.current_release_id -ne $rollback.id -or
        $historicalOne.input_hash -ne $firstRustBuild.input_hash) {
        throw 'Rollback did not republish the exact historical snapshot or preserve release history'
    }

    Write-Host '==> Project-scoped audit filtering, correlation, pagination, and RBAC tests'
    $publishedAuditResponse = Invoke-RestMethod `
        -Uri "$apiRoot/projects/$($project.id)/audit?action=release.published&created_from=2000-01-01T00:00:00Z&created_until=2100-01-01T00:00:00Z&limit=20" `
        -Headers $adminHeaders
    $publishedAudit = @($publishedAuditResponse | ForEach-Object { $_ })
    $releaseOneId = [guid]::Parse($releaseOne.id.ToString())
    $releaseTwoId = [guid]::Parse($releaseTwo.id.ToString())
    $projectIdValue = [guid]::Parse($project.id.ToString())
    $releaseOneAudits = @($publishedAudit | Where-Object { [guid]::Parse($_.entity_id.ToString()) -eq $releaseOneId })
    $releaseTwoAudits = @($publishedAudit | Where-Object { [guid]::Parse($_.entity_id.ToString()) -eq $releaseTwoId })
    $foreignProjectAudits = @($publishedAudit | Where-Object { [guid]::Parse($_.project_id.ToString()) -ne $projectIdValue })
    if ($publishedAudit.Count -lt 2 -or $releaseOneAudits.Count -ne 1 -or
        $releaseTwoAudits.Count -ne 1 -or $foreignProjectAudits.Count -ne 0) {
        $auditDiagnostic = $publishedAudit | ConvertTo-Json -Depth 5 -Compress
        throw "Audit action/time filtering did not return releases $releaseOneId and $releaseTwoId for project ${projectIdValue}: $auditDiagnostic"
    }
    $correlatedAuditResponse = Invoke-RestMethod `
        -Uri "$apiRoot/projects/$($project.id)/audit?correlation_id=$($publishedAudit[0].correlation_id)&limit=20" `
        -Headers $adminHeaders
    $correlatedAudit = @($correlatedAuditResponse | ForEach-Object { $_ })
    if ($correlatedAudit.Count -ne 1 -or $correlatedAudit[0].id -ne $publishedAudit[0].id) {
        throw 'Audit correlation filter did not return the originating event'
    }
    $auditPageResponse = Invoke-RestMethod `
        -Uri "$apiRoot/projects/$($project.id)/audit?limit=2" -Headers $adminHeaders
    $auditPage = @($auditPageResponse | ForEach-Object { $_ })
    $nextAuditPageResponse = Invoke-RestMethod `
        -Uri "$apiRoot/projects/$($project.id)/audit?limit=2&before_id=$($auditPage[-1].id)" `
        -Headers $adminHeaders
    $nextAuditPage = @($nextAuditPageResponse | ForEach-Object { $_ })
    if ($auditPage.Count -ne 2 -or $nextAuditPage.Count -eq 0 -or
        @($auditPage.id | Where-Object { $nextAuditPage.id -contains $_ }).Count -ne 0) {
        throw 'Audit cursor pagination returned a duplicate or empty next page'
    }
    $privateProject = Invoke-RestMethod -Method Post -Uri "$apiRoot/projects" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ name = 'Private Audit Project'; description = 'RBAC boundary' } | ConvertTo-Json)
    $crossProjectAuditStatus = 0
    try {
        Invoke-RestMethod -Uri "$apiRoot/projects/$($privateProject.id)/audit" `
            -Headers $viewerHeaders | Out-Null
    }
    catch { $crossProjectAuditStatus = [int]$_.Exception.Response.StatusCode }
    if ($crossProjectAuditStatus -ne 403) {
        throw "Cross-project audit query returned HTTP $crossProjectAuditStatus instead of 403"
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

    Write-Host '==> Concurrent editor conflict and 1,024-row performance budget tests'
    $concurrentValues = @{}
    $concurrentValues[$fieldId] = @{ kind = 'integer'; value = 25 }
    $concurrentRow = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/schemas/$schemaId/rows" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ row = @{
            id = [guid]::NewGuid().ToString()
            schema_id = $schemaId
            revision_id = $destructiveSchema.revision_id
            values = $concurrentValues
        }} | ConvertTo-Json -Depth 20)
    $concurrentPayload = @{ row = $concurrentRow.row; expected_version = 1 } | ConvertTo-Json -Depth 20
    $httpClient = [Net.Http.HttpClient]::new()
    $requestOne = [Net.Http.HttpRequestMessage]::new(
        [Net.Http.HttpMethod]::Put,
        "$apiRoot/projects/$($project.id)/schemas/$schemaId/rows/$($concurrentRow.row.id)"
    )
    $requestTwo = [Net.Http.HttpRequestMessage]::new(
        [Net.Http.HttpMethod]::Put,
        "$apiRoot/projects/$($project.id)/schemas/$schemaId/rows/$($concurrentRow.row.id)"
    )
    foreach ($request in @($requestOne, $requestTwo)) {
        $request.Headers.TryAddWithoutValidation('Authorization', "Bearer $($admin.token)") | Out-Null
        $request.Headers.TryAddWithoutValidation('X-CSRF-Token', $admin.csrf_token) | Out-Null
        $request.Content = [Net.Http.StringContent]::new(
            $concurrentPayload, [Text.Encoding]::UTF8, 'application/json'
        )
    }
    $concurrencyTimer = [Diagnostics.Stopwatch]::StartNew()
    $taskOne = $httpClient.SendAsync($requestOne)
    $taskTwo = $httpClient.SendAsync($requestTwo)
    $responseOne = $taskOne.GetAwaiter().GetResult()
    $responseTwo = $taskTwo.GetAwaiter().GetResult()
    $concurrencyTimer.Stop()
    $concurrentStatuses = @([int]$responseOne.StatusCode, [int]$responseTwo.StatusCode) | Sort-Object
    $responseOne.Dispose()
    $responseTwo.Dispose()
    $httpClient.Dispose()
    if (($concurrentStatuses -join ',') -ne '200,409' -or $concurrencyTimer.Elapsed.TotalSeconds -gt 5) {
        throw "Concurrent writers did not yield 200/409 within 5s: $($concurrentStatuses -join ',') in $($concurrencyTimer.Elapsed)"
    }

    $largeSql = @"
BEGIN;
CREATE TEMP TABLE quality_large_rows AS
SELECT uuidv7() AS row_id, uuidv7() AS revision_id, uuidv7() AS data_revision_id,
       ((value - 1) % 100) + 1 AS field_value
FROM generate_series(1, 1024) AS value;
INSERT INTO datahub_config_rows
    (id, schema_id, document, version, current_revision_id, created_by, updated_by)
SELECT row_id, '$schemaId',
       jsonb_build_object(
           'id', row_id::text,
           'schema_id', '$schemaId',
           'revision_id', revision_id::text,
           'values', jsonb_build_object('$fieldId', jsonb_build_object('kind', 'integer', 'value', field_value))
       ),
       1, revision_id, '$($admin.user.id)', '$($admin.user.id)'
FROM quality_large_rows;
INSERT INTO datahub_row_revisions (revision_id, row_id, version, snapshot, actor_id)
SELECT fixture.revision_id, fixture.row_id, 1, rows.document, '$($admin.user.id)'
FROM quality_large_rows AS fixture
JOIN datahub_config_rows AS rows ON rows.id = fixture.row_id;
INSERT INTO datahub_data_revisions
    (revision_id, project_id, schema_id, row_id, row_revision_id, actor_id)
SELECT data_revision_id, '$($project.id)', '$schemaId', row_id, revision_id, '$($admin.user.id)'
FROM quality_large_rows;
COMMIT;
"@
    Invoke-Checked 'Insert stable-ID large-table fixture' {
        docker compose -p $qualityProject exec -T postgres psql `
            -U datahub_quality -d datahub_quality -v ON_ERROR_STOP=1 -c $largeSql
    }
    $resyncTimer = [Diagnostics.Stopwatch]::StartNew()
    $largeResync = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/sync/resync" `
        -Headers $adminHeaders -ContentType 'application/json'
    $resyncTimer.Stop()
    if ($largeResync.projected_rows -ne 1027 -or $resyncTimer.Elapsed.TotalSeconds -gt 30) {
        throw "Large-table resync exceeded budget or lost rows: $($largeResync.projected_rows) rows in $($resyncTimer.Elapsed)"
    }
    $viewTimer = [Diagnostics.Stopwatch]::StartNew()
    $largeView = Invoke-RestMethod -Method Post `
        -Uri "$apiRoot/projects/$($project.id)/schemas/$schemaId/views" `
        -Headers $adminHeaders -ContentType 'application/json' `
        -Body (@{ block_size = 256; filters = @(); sort = @() } | ConvertTo-Json)
    $largeBlock = Invoke-RestMethod `
        -Uri "$apiRoot/table-views/$($largeView.view_id)/blocks/0" -Headers $adminHeaders
    $viewTimer.Stop()
    if ($largeView.total_rows -ne 1027 -or $largeBlock.rows.Count -ne 256 -or
        $viewTimer.Elapsed.TotalSeconds -gt 2) {
        throw "Large-table first block exceeded budget: $($largeView.total_rows)/$($largeBlock.rows.Count) in $($viewTimer.Elapsed)"
    }

    $finalMetrics = (Invoke-WebRequest -UseBasicParsing -Uri 'http://127.0.0.1:18080/metrics').Content
    if ($finalMetrics -notmatch 'datahub_outbox_events\{state="dead_lettered"\} 1' -or
        $finalMetrics -notmatch 'datahub_releases_published 1' -or
        $finalMetrics -notmatch 'datahub_http_responses_total\{class="4xx"\} [1-9]') {
        throw 'Operational metrics did not expose tested failure/release states'
    }

    Invoke-Checked 'Verify SQLx migration' {
        docker compose -p $qualityProject exec -T postgres psql `
            -U datahub_quality -d datahub_quality -v ON_ERROR_STOP=1 `
            -c 'SELECT version, description, success FROM _sqlx_migrations ORDER BY version; SELECT COUNT(*) AS schema_revisions FROM datahub_schema_revisions; SELECT COUNT(*) AS row_revisions FROM datahub_row_revisions; SELECT COUNT(*) AS data_revisions FROM datahub_data_revisions; SELECT COUNT(*) AS builds FROM datahub_jobs WHERE kind = ''build''; SELECT COUNT(*) AS artifacts FROM datahub_build_artifacts; SELECT COUNT(*) AS projected_schemas FROM datahub_projection_schemas; SELECT COUNT(*) AS projected_rows FROM datahub_projection_rows; SELECT COUNT(*) AS projection_plans FROM datahub_projection_plans; SELECT COUNT(*) AS releases FROM datahub_releases; SELECT COUNT(*) AS audit_events FROM datahub_audit_events; SELECT COUNT(*) AS outbox_events FROM datahub_outbox_events;'
    }

    Write-Host '==> Fresh-volume PostgreSQL backup and restore integrity tests'
    $backupRoot = [IO.Path]::GetFullPath((Join-Path ([IO.Path]::GetTempPath()) "datahub-backup-$([guid]::NewGuid())"))
    $backupTempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
    if (-not $backupRoot.StartsWith($backupTempRoot, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Backup test directory escaped the system temp root: $backupRoot"
    }
    New-Item -ItemType Directory -Path $backupRoot | Out-Null
    $backupFile = Join-Path $backupRoot 'datahub-quality.dump'
    $backupTimer = [Diagnostics.Stopwatch]::StartNew()
    Invoke-Checked 'Create PostgreSQL backup' {
        pwsh -NoProfile -File scripts/backup-postgres.ps1 `
            -OutputPath $backupFile -ComposeProject $qualityProject `
            -Service postgres -Database datahub_quality -User datahub_quality
    }
    Invoke-Checked 'Start fresh recovery PostgreSQL volume' {
        docker compose -p $qualityProject --profile recovery up `
            --detach postgres-restore --wait --wait-timeout 120
    }
    Invoke-Checked 'Restore PostgreSQL backup' {
        pwsh -NoProfile -File scripts/restore-postgres.ps1 `
            -InputPath $backupFile -ComposeProject $qualityProject `
            -Service postgres-restore -Database datahub_quality_restore -User datahub_quality
    }
    $backupTimer.Stop()
    if ($backupTimer.Elapsed.TotalSeconds -gt 120) {
        throw "Backup/restore exceeded 120 second budget: $($backupTimer.Elapsed)"
    }
    $integrityQuery = @'
SELECT concat_ws('|',
  (SELECT COUNT(*) FROM datahub_users),
  (SELECT COUNT(*) FROM datahub_projects),
  (SELECT COUNT(*) FROM datahub_project_members),
  (SELECT COUNT(*) FROM datahub_schemas),
  (SELECT COUNT(*) FROM datahub_schema_revisions),
  (SELECT COUNT(*) FROM datahub_config_rows),
  (SELECT COUNT(*) FROM datahub_row_revisions),
  (SELECT COUNT(*) FROM datahub_data_revisions),
  (SELECT COUNT(*) FROM datahub_formula_sets),
  (SELECT COUNT(*) FROM datahub_jobs),
  (SELECT COUNT(*) FROM datahub_build_artifacts),
  (SELECT COUNT(*) FROM datahub_projection_schemas),
  (SELECT COUNT(*) FROM datahub_projection_rows),
  (SELECT COUNT(*) FROM datahub_projection_plans),
  (SELECT COUNT(*) FROM datahub_projection_schema_versions),
  (SELECT COUNT(*) FROM datahub_environments),
  (SELECT COUNT(*) FROM datahub_releases),
  (SELECT COUNT(*) FROM datahub_audit_events),
  (SELECT COUNT(*) FROM datahub_outbox_events),
  (SELECT COUNT(*) FROM datahub_rate_limit_buckets),
  (SELECT md5(COALESCE(string_agg(input_hash, '' ORDER BY id), '')) FROM datahub_jobs WHERE kind = 'build'),
  (SELECT md5(COALESCE(string_agg(sha256, '' ORDER BY build_id, path), '')) FROM datahub_build_artifacts),
  (SELECT md5(COALESCE(string_agg(manifest::text, '' ORDER BY id), '')) FROM datahub_jobs WHERE kind = 'build'),
  (SELECT md5(COALESCE(string_agg(input_hash || manifest::text, '' ORDER BY id), '')) FROM datahub_releases)
);
'@
    $sourceIntegrity = docker compose -p $qualityProject exec -T postgres psql `
        -U datahub_quality -d datahub_quality -v ON_ERROR_STOP=1 -Atc $integrityQuery
    if ($LASTEXITCODE -ne 0) { throw 'Source integrity query failed' }
    $restoredIntegrity = docker compose -p $qualityProject exec -T postgres-restore psql `
        -U datahub_quality -d datahub_quality_restore -v ON_ERROR_STOP=1 -Atc $integrityQuery
    if ($LASTEXITCODE -ne 0 -or $sourceIntegrity.Trim() -ne $restoredIntegrity.Trim()) {
        throw "Restored durable state differs from source.`nSource: $sourceIntegrity`nRestore: $restoredIntegrity"
    }
    Invoke-Checked 'Continue writing restored database' {
        docker compose -p $qualityProject exec -T postgres-restore psql `
            -U datahub_quality -d datahub_quality_restore -v ON_ERROR_STOP=1 `
            -c "INSERT INTO datahub_system_info (key, value) VALUES ('restore-continuation', jsonb_build_object('verified', true));"
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
    if ($null -ne $compileRoot -and (Test-Path -LiteralPath $compileRoot)) {
        $resolvedCompileRoot = [IO.Path]::GetFullPath((Resolve-Path -LiteralPath $compileRoot).Path)
        $resolvedTempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
        if ($resolvedCompileRoot.StartsWith($resolvedTempRoot, [StringComparison]::OrdinalIgnoreCase) -and
            (Split-Path -Leaf $resolvedCompileRoot).StartsWith('datahub-generated-', [StringComparison]::Ordinal)) {
            Remove-Item -LiteralPath $resolvedCompileRoot -Recurse -Force
        }
        else {
            Write-Warning "Refused to remove unexpected generated-code directory: $resolvedCompileRoot"
        }
    }
    if ($null -ne $pluginTestRoot -and (Test-Path -LiteralPath $pluginTestRoot)) {
        $resolvedPluginRoot = [IO.Path]::GetFullPath((Resolve-Path -LiteralPath $pluginTestRoot).Path)
        $resolvedTempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
        if ($resolvedPluginRoot.StartsWith($resolvedTempRoot, [StringComparison]::OrdinalIgnoreCase) -and
            (Split-Path -Leaf $resolvedPluginRoot).StartsWith('datahub-plugin-', [StringComparison]::Ordinal)) {
            Remove-Item -LiteralPath $resolvedPluginRoot -Recurse -Force
        }
        else {
            Write-Warning "Refused to remove unexpected plugin test directory: $resolvedPluginRoot"
        }
    }
    if ($null -ne $backupRoot -and (Test-Path -LiteralPath $backupRoot)) {
        $resolvedBackupRoot = [IO.Path]::GetFullPath((Resolve-Path -LiteralPath $backupRoot).Path)
        $resolvedTempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
        if ($resolvedBackupRoot.StartsWith($resolvedTempRoot, [StringComparison]::OrdinalIgnoreCase) -and
            (Split-Path -Leaf $resolvedBackupRoot).StartsWith('datahub-backup-', [StringComparison]::Ordinal)) {
            Remove-Item -LiteralPath $resolvedBackupRoot -Recurse -Force
        }
        else {
            Write-Warning "Refused to remove unexpected backup test directory: $resolvedBackupRoot"
        }
    }
    Pop-Location
}
