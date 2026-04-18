# RSC P2P Backend - Arrancar servidor (Windows)
# Sin --reload: evita PermissionError [WinError 5] en CreateNamedPipe (multiprocessing).
# Recarga automática: .\run-reload.ps1 (solo si tu entorno no falla con el reloader).
# Ejecutar: .\run.ps1

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

if (-not (Test-Path ".venv")) {
    Write-Host "Primero ejecuta: .\install.ps1"
    exit 1
}

.\.venv\Scripts\Activate.ps1
uvicorn app.main:app --host 0.0.0.0 --port 8000
