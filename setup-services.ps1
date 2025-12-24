# Script para configurar PostgreSQL y Redis en Windows

Write-Host "=== Configuración de Servicios para RSC Backend ===" -ForegroundColor Cyan

# Verificar PostgreSQL
Write-Host "`n[1/3] Verificando PostgreSQL..." -ForegroundColor Yellow
$pgRunning = Test-NetConnection -ComputerName localhost -Port 5432 -InformationLevel Quiet -WarningAction SilentlyContinue

if ($pgRunning) {
    Write-Host "✓ PostgreSQL está corriendo en el puerto 5432" -ForegroundColor Green
} else {
    Write-Host "✗ PostgreSQL NO está corriendo" -ForegroundColor Red
    Write-Host "  Opciones:" -ForegroundColor Yellow
    Write-Host "  1. Instalar PostgreSQL desde: https://www.postgresql.org/download/windows/" -ForegroundColor White
    Write-Host "  2. O usar Docker: docker run --name postgres-rsc -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=rsc_db -p 5432:5432 -d postgres:14" -ForegroundColor White
}

# Verificar Redis
Write-Host "`n[2/3] Verificando Redis..." -ForegroundColor Yellow
$redisRunning = Test-NetConnection -ComputerName localhost -Port 6379 -InformationLevel Quiet -WarningAction SilentlyContinue

if ($redisRunning) {
    Write-Host "✓ Redis está corriendo en el puerto 6379" -ForegroundColor Green
} else {
    Write-Host "✗ Redis NO está corriendo" -ForegroundColor Red
    Write-Host "  Opciones:" -ForegroundColor Yellow
    Write-Host "  1. Instalar Memurai (Redis para Windows): https://www.memurai.com/" -ForegroundColor White
    Write-Host "  2. O usar WSL2 con Redis" -ForegroundColor White
    Write-Host "  3. O usar Docker: docker run --name redis-rsc -p 6379:6379 -d redis:7-alpine" -ForegroundColor White
}

# Verificar Node.js
Write-Host "`n[3/3] Verificando Node.js..." -ForegroundColor Yellow
try {
    $nodeVersion = node --version
    Write-Host "✓ Node.js instalado: $nodeVersion" -ForegroundColor Green
} catch {
    Write-Host "✗ Node.js NO está instalado" -ForegroundColor Red
    Write-Host "  Instalar desde: https://nodejs.org/" -ForegroundColor White
}

Write-Host "`n=== Resumen ===" -ForegroundColor Cyan
Write-Host "Para continuar, asegúrate de tener:" -ForegroundColor Yellow
Write-Host "  ✓ PostgreSQL corriendo (puerto 5432)" -ForegroundColor White
Write-Host "  ✓ Redis corriendo (puerto 6379)" -ForegroundColor White
Write-Host "  ✓ Base de datos 'rsc_db' creada" -ForegroundColor White
Write-Host "`nLuego ejecuta: npm run dev" -ForegroundColor Green

