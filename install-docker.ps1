# Script para instalar Docker Desktop (requiere ejecutar como Administrador)

Write-Host "=== Instalación de Docker Desktop ===" -ForegroundColor Cyan

# Verificar si se ejecuta como administrador
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "`n✗ Este script requiere permisos de Administrador" -ForegroundColor Red
    Write-Host "`nPor favor:" -ForegroundColor Yellow
    Write-Host "1. Cierra esta ventana" -ForegroundColor White
    Write-Host "2. Abre PowerShell como Administrador (clic derecho > Ejecutar como administrador)" -ForegroundColor White
    Write-Host "3. Navega a: cd $PWD" -ForegroundColor White
    Write-Host "4. Ejecuta: .\install-docker.ps1" -ForegroundColor White
    Write-Host "`nO ejecuta manualmente:" -ForegroundColor Yellow
    Write-Host "  winget install --id Docker.DockerDesktop --accept-package-agreements --accept-source-agreements" -ForegroundColor White
    exit 1
}

Write-Host "`n[1/3] Verificando si Docker ya está instalado..." -ForegroundColor Yellow
try {
    $dockerVersion = docker --version 2>&1
    Write-Host "✓ Docker ya está instalado: $dockerVersion" -ForegroundColor Green
    Write-Host "`nPuedes continuar con: .\start-services.ps1" -ForegroundColor Cyan
    exit 0
} catch {
    Write-Host "  Docker no está instalado, continuando..." -ForegroundColor Gray
}

Write-Host "`n[2/3] Instalando Docker Desktop con winget..." -ForegroundColor Yellow
Write-Host "  Esto puede tardar varios minutos..." -ForegroundColor Gray

try {
    winget install --id Docker.DockerDesktop --accept-package-agreements --accept-source-agreements
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "`n✓ Docker Desktop instalado correctamente!" -ForegroundColor Green
        Write-Host "`n[3/3] Próximos pasos:" -ForegroundColor Yellow
        Write-Host "1. REINICIA tu computadora (requerido)" -ForegroundColor White
        Write-Host "2. Después del reinicio, inicia Docker Desktop desde el menú de inicio" -ForegroundColor White
        Write-Host "3. Espera a que Docker Desktop esté completamente iniciado (ícono verde)" -ForegroundColor White
        Write-Host "4. Ejecuta: .\start-services.ps1" -ForegroundColor White
        Write-Host "5. Ejecuta: npm run dev" -ForegroundColor White
    } else {
        Write-Host "`n✗ Error durante la instalación" -ForegroundColor Red
        Write-Host "  Código de salida: $LASTEXITCODE" -ForegroundColor Yellow
    }
} catch {
    Write-Host "`n✗ Error: $_" -ForegroundColor Red
    Write-Host "`nIntenta instalar manualmente desde:" -ForegroundColor Yellow
    Write-Host "  https://www.docker.com/products/docker-desktop/" -ForegroundColor White
}

