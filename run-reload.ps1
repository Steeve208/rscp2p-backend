# RSC P2P Backend - Arrancar con recarga automática
# En Windows, el --reload de uvicorn usa multiprocessing y suele provocar:
#   PermissionError [WinError 5] CreateNamedPipe — Acceso denegado
# En Windows se usa watchmedo (watchdog): reinicia uvicorn al cambiar .py, sin ese reloader.
# Ejecutar: .\run-reload.ps1

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

if (-not (Test-Path ".venv")) {
    Write-Host "Primero ejecuta: .\install.ps1"
    exit 1
}

.\.venv\Scripts\Activate.ps1

$isWin = ($PSVersionTable.PSVersion.Major -ge 6 -and $IsWindows) -or ($env:OS -like "*Windows*")

if ($isWin) {
    $watchmedo = Join-Path $PSScriptRoot ".venv\Scripts\watchmedo.exe"
    if (-not (Test-Path $watchmedo)) {
        Write-Host "Instalando watchdog (recarga segura en Windows)..." -ForegroundColor Yellow
        pip install "watchdog>=4.0,<5"
    }
    Write-Host "Recarga con watchdog (Windows). Sin recarga: .\run.ps1" -ForegroundColor Cyan
    & $watchmedo auto-restart `
        --directory="$PSScriptRoot\app" `
        --pattern="*.py" `
        --recursive `
        -- uvicorn app.main:app --host 0.0.0.0 --port 8000
    exit $LASTEXITCODE
}

uvicorn app.main:app --reload --host 0.0.0.0 --port 8000
