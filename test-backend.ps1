# Test rapido: comprobar que el backend responde
# Ejecutar: .\test-backend.ps1
# Requiere: backend arrancado (.\run.ps1 o .\run-no-reload.ps1)

$ErrorActionPreference = "Stop"
$base = "http://localhost:8000"

Write-Host "=== Test Backend RSC P2P ===" -ForegroundColor Cyan

# 1. Raiz
Write-Host "`n1. GET / (raiz)" -ForegroundColor White
try {
    $r = Invoke-RestMethod -Uri "$base/" -Method Get -TimeoutSec 5
    Write-Host "   Respuesta: $($r | ConvertTo-Json -Compress)" -ForegroundColor Gray
    Write-Host "   OK" -ForegroundColor Green
} catch {
    Write-Host "   FALLO: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

# 2. Health
Write-Host "`n2. GET /api/health" -ForegroundColor White
try {
    $h = Invoke-RestMethod -Uri "$base/api/health" -Method Get -TimeoutSec 5
    Write-Host "   Respuesta: $($h | ConvertTo-Json -Compress)" -ForegroundColor Gray
    Write-Host "   OK" -ForegroundColor Green
} catch {
    Write-Host "   FALLO: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

Write-Host "`n=== Backend funcionando correctamente ===" -ForegroundColor Green
