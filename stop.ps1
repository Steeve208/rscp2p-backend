# Detiene el backend que escucha en el puerto configurado (por defecto 8000).
# Útil cuando Ctrl+C no para el proceso (terminal en segundo plano, reloader, varias ventanas).
# Ejecutar: .\stop.ps1
# Otro puerto: .\stop.ps1 -Port 8001

param(
    [int]$Port = 8000
)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

function Get-PidsOnPort {
    param([int]$LocalPort)
    # Quien realmente escucha (evita filas de TIME_WAIT u otros estados sin listener)
    $listen = Get-NetTCPConnection -LocalPort $LocalPort -State Listen -ErrorAction SilentlyContinue
    if ($listen) {
        return @($listen | Select-Object -ExpandProperty OwningProcess -Unique | Where-Object { $_ -gt 0 })
    }
    # Fallback: cualquier conexión asociada al puerto
    $any = Get-NetTCPConnection -LocalPort $LocalPort -ErrorAction SilentlyContinue
    if ($any) {
        return @($any | Select-Object -ExpandProperty OwningProcess -Unique | Where-Object { $_ -gt 0 })
    }
    return @()
}

# Procesos python que ejecutan uvicorn con app.main (hijos del reloader u otra terminal)
function Get-UvicornMainPids {
    Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
        Where-Object {
            $_.Name -match '^(python|pythonw)(\d+)?\.exe$' -and
            $_.CommandLine -and
            $_.CommandLine -match 'uvicorn' -and
            $_.CommandLine -match 'app\.main:app'
        } |
        ForEach-Object { [int]$_.ProcessId }
}

$pidsSet = [System.Collections.Generic.HashSet[int]]::new()
foreach ($procId in Get-PidsOnPort -LocalPort $Port) { [void]$pidsSet.Add($procId) }
foreach ($procId in Get-UvicornMainPids) { [void]$pidsSet.Add($procId) }
$pids = @($pidsSet)

if ($pids.Count -eq 0) {
    Write-Host "No hay proceso del backend detectado (puerto $Port ni uvicorn app.main)." -ForegroundColor Yellow
    exit 0
}

foreach ($procId in $pids) {
    try {
        $p = Get-Process -Id $procId -ErrorAction Stop
        Write-Host "Deteniendo PID $procId ($($p.ProcessName))..." -ForegroundColor Cyan
        Stop-Process -Id $procId -Force
    }
    catch {
        Write-Host "PID ${procId}: $($_.Exception.Message)" -ForegroundColor DarkYellow
    }
}

Start-Sleep -Milliseconds 400
$still = Get-PidsOnPort -LocalPort $Port
if ($still -and $still.Count -gt 0) {
    Write-Host "El puerto $Port sigue en uso. Ejecuta de nuevo .\stop.ps1 o revisa con:" -ForegroundColor Yellow
    Write-Host "  Get-NetTCPConnection -LocalPort $Port | Format-Table -AutoSize" -ForegroundColor Gray
    exit 1
}

Write-Host "Backend detenido (puerto $Port libre)." -ForegroundColor Green
