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

    Invoke-Checked 'Verify SQLx migration' {
        docker compose -p $qualityProject exec -T postgres psql `
            -U datahub_quality -d datahub_quality -v ON_ERROR_STOP=1 `
            -c 'SELECT version, description, success FROM _sqlx_migrations;'
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
