# Script para subir el backend a GitHub
# Este script detecta Git y sube todos los archivos al repositorio

Write-Host "=== Subiendo Backend a GitHub ===" -ForegroundColor Cyan

# Función para encontrar Git
function Find-Git {
    # Actualizar PATH con ubicaciones comunes
    $commonPaths = @(
        "C:\Program Files\Git\bin",
        "C:\Program Files\Git\cmd",
        "C:\Program Files (x86)\Git\bin",
        "$env:LOCALAPPDATA\Programs\Git\bin",
        "$env:USERPROFILE\AppData\Local\Programs\Git\bin"
    )
    
    foreach ($path in $commonPaths) {
        if (Test-Path $path) {
            $env:Path = "$path;$env:Path"
        }
    }
    
    # Buscar ejecutables específicos
    $gitPaths = @(
        "C:\Program Files\Git\bin\git.exe",
        "C:\Program Files\Git\cmd\git.exe",
        "C:\Program Files (x86)\Git\bin\git.exe",
        "$env:LOCALAPPDATA\Programs\Git\bin\git.exe",
        "$env:USERPROFILE\AppData\Local\Programs\Git\bin\git.exe"
    )
    
    foreach ($path in $gitPaths) {
        if (Test-Path $path) {
            return $path
        }
    }
    
    # Buscar en PATH actualizado
    $gitInPath = Get-Command git -ErrorAction SilentlyContinue
    if ($gitInPath) {
        return $gitInPath.Source
    }
    
    # Intentar instalar con winget si está disponible
    if (Get-Command winget -ErrorAction SilentlyContinue) {
        Write-Host "Intentando instalar Git con winget..." -ForegroundColor Yellow
        try {
            winget install --id Git.Git -e --source winget --accept-package-agreements --accept-source-agreements --silent
            Start-Sleep -Seconds 5
            # Actualizar PATH después de instalación
            $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
            $gitInPath = Get-Command git -ErrorAction SilentlyContinue
            if ($gitInPath) {
                return $gitInPath.Source
            }
        } catch {
            Write-Host "Error al instalar con winget: $_" -ForegroundColor Red
        }
    }
    
    return $null
}

# Buscar Git
$gitExe = Find-Git

if (-not $gitExe) {
    Write-Host "ERROR: Git no está instalado o no se encuentra en el PATH." -ForegroundColor Red
    Write-Host "Por favor instala Git desde: https://git-scm.com/download/win" -ForegroundColor Yellow
    Write-Host "O ejecuta: winget install Git.Git" -ForegroundColor Yellow
    exit 1
}

Write-Host "Git encontrado en: $gitExe" -ForegroundColor Green

# Usar Git encontrado
$env:Path = (Split-Path $gitExe -Parent) + ";" + $env:Path

# Verificar versión
Write-Host "`nVerificando versión de Git..." -ForegroundColor Cyan
& git --version

# Configurar Git si no está configurado
Write-Host "`nConfigurando Git..." -ForegroundColor Cyan
$userName = & git config --global user.name 2>$null
$userEmail = & git config --global user.email 2>$null

if (-not $userName) {
    Write-Host "Configurando usuario de Git..." -ForegroundColor Yellow
    & git config --global user.name "Steeve208"
}

if (-not $userEmail) {
    Write-Host "Configurando email de Git..." -ForegroundColor Yellow
    & git config --global user.email "steeve208@users.noreply.github.com"
}

# Inicializar repositorio si no existe
if (-not (Test-Path .git)) {
    Write-Host "`nInicializando repositorio Git..." -ForegroundColor Cyan
    & git init
}

# Verificar remoto
Write-Host "`nConfigurando remoto..." -ForegroundColor Cyan
$remoteUrl = & git remote get-url origin 2>$null
if ($LASTEXITCODE -ne 0) {
    & git remote add origin https://github.com/Steeve208/rscp2p-backend.git
    Write-Host "Remoto agregado" -ForegroundColor Green
} else {
    & git remote set-url origin https://github.com/Steeve208/rscp2p-backend.git
    Write-Host "Remoto actualizado" -ForegroundColor Green
}

# Agregar todos los archivos
Write-Host "`nAgregando archivos al staging..." -ForegroundColor Cyan
& git add .

# Verificar si hay cambios
$status = & git status --porcelain
if ($status) {
    Write-Host "`nHaciendo commit..." -ForegroundColor Cyan
    & git commit -m "Initial commit: Backend P2P"
    
    # Subir al repositorio
    Write-Host "`nSubiendo al repositorio..." -ForegroundColor Cyan
    & git branch -M main
    & git push -u origin main --force
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "`n¡Éxito! El código ha sido subido a GitHub." -ForegroundColor Green
        Write-Host "Repositorio: https://github.com/Steeve208/rscp2p-backend" -ForegroundColor Cyan
    } else {
        Write-Host "`nError al subir. Verifica tus credenciales de GitHub." -ForegroundColor Red
        Write-Host "Puede que necesites configurar un token de acceso personal." -ForegroundColor Yellow
    }
} else {
    Write-Host "`nNo hay cambios para commitear." -ForegroundColor Yellow
    Write-Host "Verificando si hay commits locales para subir..." -ForegroundColor Cyan
    $localCommits = & git log origin/main..HEAD --oneline 2>$null
    if ($localCommits) {
        & git push -u origin main
    } else {
        Write-Host "Todo está sincronizado." -ForegroundColor Green
    }
}

Write-Host "`n=== Proceso completado ===" -ForegroundColor Cyan

