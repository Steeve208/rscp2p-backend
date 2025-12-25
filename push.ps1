# Script para actualizar git con todos los cambios
Write-Host "=== Agregando todos los archivos ===" -ForegroundColor Yellow
git add .

Write-Host "`n=== Estado del repositorio ===" -ForegroundColor Yellow
git status

Write-Host "`n=== Haciendo commit ===" -ForegroundColor Yellow
git commit -m "Actualización: nuevos scripts y documentación" -q

Write-Host "`n=== Subiendo al repositorio ===" -ForegroundColor Yellow
git push origin main --force

Write-Host "`n=== Verificando ===" -ForegroundColor Yellow
git status
git log --oneline -1



