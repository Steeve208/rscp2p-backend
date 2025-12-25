# Script para subir scripts de configuración de DB al servidor
# Uso: .\subir-scripts-db.ps1

param(
    [Parameter(Mandatory=$false)]
    [string]$ServerIP = "",
    
    [Parameter(Mandatory=$false)]
    [string]$User = "root",
    
    [Parameter(Mandatory=$false)]
    [string]$RemotePath = "/var/www/p2prsc-backend"
)

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "Subir Scripts de Configuración DB al Servidor" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

# Si no se proporciona IP, pedirla
if ([string]::IsNullOrEmpty($ServerIP)) {
    $ServerIP = Read-Host "Ingresa la IP del servidor (ej: 123.45.67.89)"
}

if ([string]::IsNullOrEmpty($ServerIP)) {
    Write-Host "Error: Se requiere la IP del servidor" -ForegroundColor Red
    exit 1
}

Write-Host "Servidor: $User@$ServerIP" -ForegroundColor Yellow
Write-Host "Ruta remota: $RemotePath" -ForegroundColor Yellow
Write-Host ""

# Archivos a subir
$files = @(
    "setup-postgresql.sh",
    "verificar-db.sh",
    "verificar-redis.sh",
    "GUIA-CONFIGURACION-DB.md",
    "COMANDOS-RAPIDOS-DB.md",
    "ARQUITECTURA-P2P.md"
)

Write-Host "Archivos a subir:" -ForegroundColor Green
foreach ($file in $files) {
    if (Test-Path $file) {
        Write-Host "  ✓ $file" -ForegroundColor Green
    } else {
        Write-Host "  ✗ $file (no encontrado)" -ForegroundColor Red
    }
}
Write-Host ""

# Confirmar
$confirm = Read-Host "¿Continuar? (S/N)"
if ($confirm -ne "S" -and $confirm -ne "s" -and $confirm -ne "Y" -and $confirm -ne "y") {
    Write-Host "Cancelado." -ForegroundColor Yellow
    exit 0
}

Write-Host ""
Write-Host "Subiendo archivos..." -ForegroundColor Cyan

# Subir cada archivo
foreach ($file in $files) {
    if (Test-Path $file) {
        Write-Host "Subiendo $file..." -ForegroundColor Yellow
        try {
            scp $file "${User}@${ServerIP}:${RemotePath}/$file"
            Write-Host "  ✓ $file subido correctamente" -ForegroundColor Green
        } catch {
            Write-Host "  ✗ Error al subir $file : $_" -ForegroundColor Red
        }
    }
}

Write-Host ""
Write-Host "============================================" -ForegroundColor Cyan
Write-Host "✓ Archivos subidos" -ForegroundColor Green
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Próximos pasos en el servidor:" -ForegroundColor Yellow
Write-Host "  1. cd $RemotePath" -ForegroundColor White
Write-Host "  2. chmod +x setup-postgresql.sh verificar-db.sh verificar-redis.sh" -ForegroundColor White
Write-Host "  3. ./setup-postgresql.sh" -ForegroundColor White
Write-Host ""

