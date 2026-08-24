[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string] $InputPath,
    [string] $ComposeProject = 'datahub',
    [string] $Service = 'postgres-restore',
    [string] $Database,
    [string] $User = $env:POSTGRES_USER
)

$ErrorActionPreference = 'Stop'
$absoluteInput = [IO.Path]::GetFullPath($InputPath)
if (-not (Test-Path -LiteralPath $absoluteInput -PathType Leaf)) {
    throw "Backup file does not exist: $absoluteInput"
}
if ([string]::IsNullOrWhiteSpace($Database)) {
    $Database = "$($env:POSTGRES_DB)_restore"
}
if ([string]::IsNullOrWhiteSpace($Database) -or [string]::IsNullOrWhiteSpace($User)) {
    throw 'Database and User are required.'
}

$container = (docker compose -p $ComposeProject ps -q $Service).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($container)) {
    throw "Compose recovery service is not running: $Service"
}
$existing = docker compose -p $ComposeProject exec -T $Service psql `
    --username=$User --dbname=$Database --tuples-only --no-align `
    --command="SELECT COUNT(*) FROM pg_catalog.pg_tables WHERE schemaname = 'public';"
if ($LASTEXITCODE -ne 0) { throw 'Could not inspect restore target' }
if ([int]$existing.Trim() -ne 0) {
    throw 'Restore target is not empty; refusing to overwrite it.'
}

$remote = "/tmp/datahub-restore-$([guid]::NewGuid().ToString('N')).dump"
try {
    docker compose -p $ComposeProject cp $absoluteInput "${Service}:$remote"
    if ($LASTEXITCODE -ne 0) { throw 'docker compose cp failed' }
    docker compose -p $ComposeProject exec -T $Service pg_restore `
        --exit-on-error --no-owner --no-acl --username=$User --dbname=$Database $remote
    if ($LASTEXITCODE -ne 0) { throw 'pg_restore failed' }
}
finally {
    docker compose -p $ComposeProject exec -T $Service rm -f -- $remote 2>$null | Out-Null
}
Write-Host "PostgreSQL backup restored into $Service/$Database"
