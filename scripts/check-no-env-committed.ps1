# Run before push: ensures .env is not tracked by git.
$ErrorActionPreference = "Stop"
$tracked = git ls-files ".env", ".env.*" 2>$null | Where-Object { $_ -notmatch '\.env\.example$' }
if ($tracked) {
    Write-Error "BLOCKED: secret env files are tracked by git:`n$($tracked -join "`n")"
}
Write-Host "OK — .env is not in git"
