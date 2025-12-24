# Script para iniciar PostgreSQL y Redis con Docker

Write-Host "=== Iniciando Servicios con Docker ===" -ForegroundColor Cyan

# Verificar Docker
Write-Host "`n[1/3] Verificando Docker..." -ForegroundColor Yellow
try {
    $dockerVersion = docker --version 2>&1
    Write-Host "✓ Docker encontrado: $dockerVersion" -ForegroundColor Green
} catch {
    Write-Host "✗ Docker NO está instalado" -ForegroundColor Red
    Write-Host "`nPor favor instala Docker Desktop desde:" -ForegroundColor Yellow
    Write-Host "  https://www.docker.com/products/docker-desktop/" -ForegroundColor White
    Write-Host "`nDespués de instalar Docker Desktop:" -ForegroundColor Yellow
    Write-Host "  1. Inicia Docker Desktop" -ForegroundColor White
    Write-Host "  2. Ejecuta este script nuevamente: .\start-services.ps1" -ForegroundColor White
    exit 1
}

# Verificar si Docker está corriendo
Write-Host "`n[2/3] Verificando si Docker está corriendo..." -ForegroundColor Yellow
try {
    docker ps > $null 2>&1
    Write-Host "✓ Docker está corriendo" -ForegroundColor Green
} catch {
    Write-Host "✗ Docker NO está corriendo" -ForegroundColor Red
    Write-Host "  Por favor inicia Docker Desktop" -ForegroundColor Yellow
    exit 1
}

# Iniciar servicios
Write-Host "`n[3/3] Iniciando PostgreSQL y Redis..." -ForegroundColor Yellow

# Detener contenedores existentes si existen
Write-Host "  Deteniendo contenedores existentes..." -ForegroundColor Gray
docker-compose down 2>&1 | Out-Null

# Iniciar servicios
Write-Host "  Iniciando servicios..." -ForegroundColor Gray
docker-compose up -d

if ($LASTEXITCODE -eq 0) {
    Write-Host "`n✓ Servicios iniciados correctamente!" -ForegroundColor Green
    Write-Host "`nServicios disponibles en:" -ForegroundColor Cyan
    Write-Host "  PostgreSQL: localhost:5432" -ForegroundColor White
    Write-Host "  Redis: localhost:6379" -ForegroundColor White
    Write-Host "`nPara ver los logs:" -ForegroundColor Yellow
    Write-Host "  docker-compose logs -f" -ForegroundColor White
    Write-Host "`nPara detener los servicios:" -ForegroundColor Yellow
    Write-Host "  docker-compose down" -ForegroundColor White
    Write-Host "`nAhora puedes ejecutar el backend:" -ForegroundColor Green
    Write-Host "  npm run dev" -ForegroundColor White
} else {
    Write-Host "`n✗ Error al iniciar servicios" -ForegroundColor Red
    Write-Host "  Verifica los logs con: docker-compose logs" -ForegroundColor Yellow
}

