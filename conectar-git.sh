#!/bin/bash
cd /d/p2p-backend

echo "=== Conectando repositorio ==="
git remote set-url origin https://github.com/Steeve208/rscp2p-backend.git
git remote -v

echo ""
echo "=== Estado del repositorio ==="
git status

echo ""
echo "=== Agregando todos los archivos ==="
git add .

echo ""
echo "=== Haciendo commit ==="
git commit -m "Initial commit: Backend P2P para RSC Finance" || echo "Sin cambios para commitear"

echo ""
echo "=== Subiendo al repositorio ==="
git push -u origin main --force

echo ""
echo "=== Verificando ==="
git status
git log --oneline -1



