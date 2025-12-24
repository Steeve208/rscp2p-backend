# Script para verificar e iniciar Memurai

Write-Host "=== Verificación de Memurai ===" -ForegroundColor Cyan

# Buscar Memurai en ubicaciones comunes
$locations = @(
    "D:\Program Files\Memurai",
    "D:\Memurai",
    "C:\Program Files\Memurai",
    "C:\Program Files (x86)\Memurai",
    "$env:ProgramFiles\Memurai",
    "$env:ProgramFiles(x86)\Memurai"
)

Write-Host "`n[1/3] Buscando Memurai..." -ForegroundColor Yellow
$found = $false
foreach ($location in $locations) {
    if (Test-Path $location) {
        Write-Host "  ✓ Encontrado en: $location" -ForegroundColor Green
        $found = $true
        
        # Buscar ejecutable
        $exe = Get-ChildItem -Path $location -Filter "memurai-server.exe" -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($exe) {
            Write-Host "  ✓ Ejecutable encontrado: $($exe.FullName)" -ForegroundColor Green
        }
    }
}

if (-not $found) {
    Write-Host "  ✗ Memurai no encontrado en ubicaciones estándar" -ForegroundColor Red
}

# Verificar servicios
Write-Host "`n[2/3] Verificando servicios..." -ForegroundColor Yellow
$services = Get-Service | Where-Object {$_.DisplayName -like "*memurai*" -or $_.DisplayName -like "*redis*"}
if ($services) {
    foreach ($service in $services) {
        Write-Host "  Servicio: $($service.DisplayName) - Estado: $($service.Status)" -ForegroundColor $(if ($service.Status -eq 'Running') {'Green'} else {'Yellow'})
    }
} else {
    Write-Host "  ✗ No se encontraron servicios de Memurai/Redis" -ForegroundColor Red
}

# Verificar puerto
Write-Host "`n[3/3] Verificando puerto 6379..." -ForegroundColor Yellow
$portOpen = Test-NetConnection -ComputerName localhost -Port 6379 -InformationLevel Quiet -WarningAction SilentlyContinue
if ($portOpen) {
    Write-Host "  ✓ Puerto 6379 está abierto" -ForegroundColor Green
} else {
    Write-Host "  ✗ Puerto 6379 no está en uso" -ForegroundColor Red
}

Write-Host "`n=== Resumen ===" -ForegroundColor Cyan
if ($found -or $services -or $portOpen) {
    Write-Host "Memurai parece estar instalado pero no está corriendo" -ForegroundColor Yellow
    Write-Host "`nPara iniciar Memurai:" -ForegroundColor Yellow
    Write-Host "1. Busca 'Memurai' en el menú de inicio" -ForegroundColor White
    Write-Host "2. O ejecuta el servicio desde 'Servicios' (services.msc)" -ForegroundColor White
    Write-Host "3. O ejecuta manualmente: memurai-server.exe" -ForegroundColor White
} else {
    Write-Host "Memurai no está instalado o no se puede encontrar" -ForegroundColor Red
    Write-Host "`nInstala Memurai desde: https://www.memurai.com/get-memurai" -ForegroundColor Yellow
}



