[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string] $OutputPath,
    [string] $ComposeProject = 'datahub',
    [string] $Service = 'postgres',
    [string] $Database = $env:POSTGRES_DB,
    [string] $User = $env:POSTGRES_USER
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Database) -or [string]::IsNullOrWhiteSpace($User)) {
    throw 'Database and User are required (parameters or POSTGRES_DB/POSTGRES_USER).'
}

$absoluteOutput = [IO.Path]::GetFullPath($OutputPath)
$parent = Split-Path -Parent $absoluteOutput
if (-not (Test-Path -LiteralPath $parent -PathType Container)) {
    throw "Backup parent directory does not exist: $parent"
}
if (Test-Path -LiteralPath $absoluteOutput -PathType Container) {
    throw "Backup output is a directory: $absoluteOutput"
}

$container = (docker compose -p $ComposeProject ps -q $Service).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($container)) {
    throw "Compose service is not running: $Service"
}
$remote = "/tmp/datahub-backup-$([guid]::NewGuid().ToString('N')).dump"
try {
    docker compose -p $ComposeProject exec -T $Service `
        pg_dump --format=custom --no-owner --no-acl --file=$remote --username=$User $Database
    if ($LASTEXITCODE -ne 0) { throw 'pg_dump failed' }
    docker compose -p $ComposeProject cp "${Service}:$remote" $absoluteOutput
    if ($LASTEXITCODE -ne 0) { throw 'docker compose cp failed' }
}
finally {
    docker compose -p $ComposeProject exec -T $Service rm -f -- $remote 2>$null | Out-Null
}

$backup = Get-Item -LiteralPath $absoluteOutput
if ($backup.Length -eq 0) { throw 'Backup file is empty' }
Write-Host "PostgreSQL backup created: $absoluteOutput ($($backup.Length) bytes)"
