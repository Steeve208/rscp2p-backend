#!/bin/bash

# ============================================
# Script de Instalación y Configuración de Redis
# para RSC Finance - Sistema P2P Wallet-to-Wallet
# ============================================

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "============================================"
echo "Instalación de Redis para RSC Finance"
echo "Sistema P2P Wallet-to-Wallet"
echo "============================================"
echo ""
echo "ℹ️  Redis es CRÍTICO para:"
echo "   - Sesiones JWT (autenticación wallet-based)"
echo "   - Rate limiting por wallet address"
echo "   - Nonces temporales para firmas"
echo "   - Locks distribuidos"
echo ""

# Verificar si Redis ya está instalado
if command -v redis-server &> /dev/null; then
    echo -e "${YELLOW}⚠️  Redis ya está instalado${NC}"
    systemctl status redis --no-pager || true
    echo ""
    read -p "¿Deseas reinstalar/configurar Redis? (s/N): " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Ss]$ ]]; then
        echo "Saltando instalación..."
        exit 0
    fi
fi

# Actualizar paquetes
echo -e "${YELLOW}[1/4]${NC} Actualizando paquetes..."
apt update -y

# Instalar Redis
echo -e "${YELLOW}[2/4]${NC} Instalando Redis..."
apt install redis-server -y

# Configurar Redis
echo -e "${YELLOW}[3/4]${NC} Configurando Redis..."

# Backup de configuración
REDIS_CONF="/etc/redis/redis.conf"
if [ -f "$REDIS_CONF" ]; then
    cp "$REDIS_CONF" "${REDIS_CONF}.backup.$(date +%Y%m%d_%H%M%S)"
fi

# Configurar Redis para producción
# Permitir conexiones desde localhost
sed -i 's/^bind 127.0.0.1 ::1/bind 127.0.0.1/' "$REDIS_CONF" || sed -i 's/^# bind 127.0.0.1/bind 127.0.0.1/' "$REDIS_CONF"

# Configurar memoria máxima (ajustar según tu servidor)
# Por defecto: 256MB, puedes cambiarlo si necesitas más
if ! grep -q "^maxmemory" "$REDIS_CONF"; then
    echo "maxmemory 256mb" >> "$REDIS_CONF"
    echo "maxmemory-policy allkeys-lru" >> "$REDIS_CONF"
fi

# Habilitar persistencia (opcional, pero recomendado)
sed -i 's/^save 900 1/save 900 1/' "$REDIS_CONF"
sed -i 's/^save 300 10/save 300 10/' "$REDIS_CONF"
sed -i 's/^save 60 10000/save 60 10000/' "$REDIS_CONF"

# Iniciar y habilitar Redis
echo -e "${YELLOW}[4/4]${NC} Iniciando Redis..."
systemctl restart redis-server
systemctl enable redis-server

# Esperar un momento para que Redis inicie
sleep 2

# Verificar que Redis está funcionando
if redis-cli ping > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Redis instalado y funcionando${NC}"
    echo ""
    
    # Mostrar información
    echo "Información de Redis:"
    redis-cli info server | grep -E "redis_version|os|uptime_in_seconds" | head -3
    echo ""
    
    # Probar operaciones críticas
    echo "Probando operaciones..."
    TEST_KEY="test:install:$(date +%s)"
    if redis-cli set "$TEST_KEY" "test" EX 5 > /dev/null 2>&1; then
        if [ "$(redis-cli get "$TEST_KEY" 2>/dev/null)" = "test" ]; then
            redis-cli del "$TEST_KEY" > /dev/null 2>&1
            echo -e "${GREEN}✓ Operaciones funcionando correctamente${NC}"
        fi
    fi
    echo ""
    
    echo "============================================"
    echo -e "${GREEN}✓ Redis configurado exitosamente${NC}"
    echo "============================================"
    echo ""
    echo "Redis está listo para:"
    echo "  ✓ Sesiones JWT (autenticación wallet-based)"
    echo "  ✓ Rate limiting por wallet address"
    echo "  ✓ Nonces temporales para firmas"
    echo "  ✓ Locks distribuidos"
    echo ""
    echo "Estado:"
    systemctl status redis-server --no-pager | head -5
    echo ""
    echo "Próximos pasos:"
    echo "  1. Verifica el .env: REDIS_HOST=localhost, REDIS_PORT=6379"
    echo "  2. Ejecuta las migraciones: npm run migration:run"
    echo "  3. Reinicia el backend: pm2 restart p2p-rsc-backend"
    echo ""
else
    echo -e "${RED}✗ Error al configurar Redis${NC}"
    echo "Revisa los logs: journalctl -u redis-server -n 20"
    exit 1
fi

