[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$patterns = @(
    'gh[opusr]_[A-Za-z0-9_]{20,}',
    'AKIA[0-9A-Z]{16}',
    '-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----',
    'postgres(ql)?://[^:$\s]+:[^@$\s]{8,}@'
)
$tracked = @(git ls-files --cached --others --exclude-standard)
if ($LASTEXITCODE -ne 0) { throw 'git ls-files inventory failed' }
$findings = @()
foreach ($path in $tracked) {
    if ($path -eq '.env.example' -or -not (Test-Path -LiteralPath $path -PathType Leaf)) { continue }
    $text = [IO.File]::ReadAllText([IO.Path]::GetFullPath($path))
    foreach ($pattern in $patterns) {
        if ($text -match $pattern) { $findings += "$path matched $pattern" }
    }
}
if ($findings.Count -gt 0) {
    $findings | ForEach-Object { Write-Error $_ }
    throw 'Potential committed secret material detected.'
}
Write-Host "Secret scan passed for $($tracked.Count) tracked files."
