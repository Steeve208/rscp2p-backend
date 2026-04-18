# RSC P2P Backend - Instalación (Windows)
# Ejecutar: .\install.ps1
# Busca Python en PATH o en rutas habituales (AppData, Program Files)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

function Find-PythonExe {
    $py = Get-Command python -ErrorAction SilentlyContinue
    if ($py) { return $py.Source }
    $py = Get-Command py -ErrorAction SilentlyContinue
    if ($py) { return $py.Source }
    $paths = @(
        "$env:LOCALAPPDATA\Programs\Python\Python312\python.exe",
        "$env:LOCALAPPDATA\Programs\Python\Python311\python.exe",
        "$env:LOCALAPPDATA\Programs\Python\Python310\python.exe",
        "C:\Program Files\Python312\python.exe",
        "C:\Program Files\Python311\python.exe"
    )
    foreach ($p in $paths) {
        if (Test-Path $p) { return $p }
    }
    return $null
}

$pythonExe = Find-PythonExe
if (-not $pythonExe) {
    Write-Host "Python no encontrado. Instala desde https://www.python.org/downloads/" -ForegroundColor Red
    exit 1
}
Write-Host "Usando: $pythonExe" -ForegroundColor Green

if (-not (Test-Path ".venv")) {
    & $pythonExe -m venv .venv
}

# Activar e instalar (pip del venv con python -m evita ERROR al actualizar pip en Windows)
.\.venv\Scripts\Activate.ps1
$venvPy = Join-Path $PSScriptRoot '.venv\Scripts\python.exe'
& $venvPy -m pip install --upgrade pip
& $venvPy -m pip install -r requirements.txt

Write-Host "Listo. Para arrancar: .\run.ps1  |  Para detener: .\stop.ps1"
