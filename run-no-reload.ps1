# RSC P2P Backend - Arrancar servidor SIN recarga automática (Windows)
# Usar si .\run.ps1 da "PermissionError" o "Acceso denegado" por el reloader.
# Ejecutar: .\run-no-reload.ps1

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

if (-not (Test-Path ".venv")) {
    Write-Host "Primero ejecuta: .\install.ps1"
    exit 1
}

.\.venv\Scripts\Activate.ps1
uvicorn app.main:app --host 0.0.0.0 --port 8000
