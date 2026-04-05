# RSC P2P Backend - Arrancar servidor (Windows)
# Ejecutar: .\run.ps1

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

if (-not (Test-Path ".venv")) {
    Write-Host "Primero ejecuta: .\install.ps1"
    exit 1
}

.\.venv\Scripts\Activate.ps1
uvicorn app.main:app --reload --host 0.0.0.0 --port 8000
