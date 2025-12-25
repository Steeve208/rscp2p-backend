#!/bin/bash

# ============================================
# Script de Verificación de Redis
# CRÍTICO para autenticación wallet-based
# ============================================

set -e

# Colores
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "============================================"
echo "Verificación de Redis para RSC Finance"
echo "Sistema P2P Wallet-to-Wallet"
echo "============================================"
echo ""
echo "ℹ️  Redis es CRÍTICO para:"
echo "   - Sesiones JWT (autenticación wallet-based)"
echo "   - Rate limiting por wallet address"
echo "   - Nonces temporales para firmas"
echo "   - Locks distribuidos"
echo ""

# Cargar variables de entorno
if [ -f "/var/www/p2prsc-backend/.env" ]; then
    export $(cat /var/www/p2prsc-backend/.env | grep -v '^#' | xargs)
else
    echo -e "${YELLOW}⚠️  No se encontró .env, usando valores por defecto${NC}"
    REDIS_HOST=${REDIS_HOST:-localhost}
    REDIS_PORT=${REDIS_PORT:-6379}
fi

REDIS_HOST=${REDIS_HOST:-localhost}
REDIS_PORT=${REDIS_PORT:-6379}
REDIS_PASSWORD=${REDIS_PASSWORD:-}

echo "Intentando conectar a Redis:"
echo "  Host: $REDIS_HOST"
echo "  Puerto: $REDIS_PORT"
echo "  Password: ${REDIS_PASSWORD:+***configurada***}${REDIS_PASSWORD:-no configurada}"
echo ""

# Verificar si redis-cli está instalado
if ! command -v redis-cli &> /dev/null; then
    echo -e "${RED}✗ redis-cli no está instalado${NC}"
    echo ""
    echo "Instala Redis client:"
    echo "  apt install redis-tools -y"
    exit 1
fi

# Construir comando de conexión
REDIS_CMD="redis-cli -h $REDIS_HOST -p $REDIS_PORT"
if [ -n "$REDIS_PASSWORD" ]; then
    REDIS_CMD="$REDIS_CMD -a $REDIS_PASSWORD"
fi

# Verificar conexión
echo -e "${YELLOW}[1/3]${NC} Verificando conexión..."
if $REDIS_CMD ping > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Conexión exitosa${NC}"
else
    echo -e "${RED}✗ Error al conectar a Redis${NC}"
    echo ""
    echo "Posibles causas:"
    echo "  1. Redis no está corriendo: systemctl status redis"
    echo "  2. Host o puerto incorrectos en .env"
    echo "  3. Password incorrecta"
    echo "  4. Firewall bloqueando conexión"
    exit 1
fi
echo ""

# Verificar información del servidor
echo -e "${YELLOW}[2/3]${NC} Información del servidor Redis..."
INFO=$($REDIS_CMD info server 2>/dev/null | grep -E "redis_version|os|uptime_in_days" | head -3)
if [ -n "$INFO" ]; then
    echo "$INFO" | while IFS= read -r line; do
        echo "  $line"
    done
else
    echo -e "${YELLOW}⚠️  No se pudo obtener información del servidor${NC}"
fi
echo ""

# Probar operaciones críticas
echo -e "${YELLOW}[3/3]${NC} Probando operaciones críticas..."

# Test 1: SET/GET (sesiones)
TEST_KEY="test:session:$(date +%s)"
if $REDIS_CMD set "$TEST_KEY" "test_value" EX 10 > /dev/null 2>&1; then
    if [ "$($REDIS_CMD get "$TEST_KEY" 2>/dev/null)" = "test_value" ]; then
        echo -e "${GREEN}✓ SET/GET funcionando (sesiones JWT)${NC}"
        $REDIS_CMD del "$TEST_KEY" > /dev/null 2>&1
    else
        echo -e "${RED}✗ GET no funciona correctamente${NC}"
    fi
else
    echo -e "${RED}✗ SET no funciona${NC}"
fi

# Test 2: EXPIRE (TTL para nonces y rate limiting)
TEST_KEY="test:ttl:$(date +%s)"
if $REDIS_CMD set "$TEST_KEY" "test" EX 5 > /dev/null 2>&1; then
    TTL=$($REDIS_CMD ttl "$TEST_KEY" 2>/dev/null)
    if [ "$TTL" -gt 0 ] && [ "$TTL" -le 5 ]; then
        echo -e "${GREEN}✓ EXPIRE funcionando (nonces y rate limiting)${NC}"
        $REDIS_CMD del "$TEST_KEY" > /dev/null 2>&1
    else
        echo -e "${RED}✗ TTL no funciona correctamente${NC}"
    fi
else
    echo -e "${RED}✗ EXPIRE no funciona${NC}"
fi

# Test 3: INCR (rate limiting)
TEST_KEY="test:ratelimit:$(date +%s)"
if $REDIS_CMD incr "$TEST_KEY" > /dev/null 2>&1; then
    VALUE=$($REDIS_CMD get "$TEST_KEY" 2>/dev/null)
    if [ "$VALUE" = "1" ]; then
        echo -e "${GREEN}✓ INCR funcionando (rate limiting por wallet)${NC}"
        $REDIS_CMD del "$TEST_KEY" > /dev/null 2>&1
    else
        echo -e "${RED}✗ INCR no funciona correctamente${NC}"
    fi
else
    echo -e "${RED}✗ INCR no funciona${NC}"
fi

echo ""

# Resumen final
echo "============================================"
echo -e "${GREEN}✓ Verificación completada${NC}"
echo "============================================"
echo ""
echo "Redis está listo para:"
echo "  ✓ Sesiones JWT (autenticación wallet-based)"
echo "  ✓ Rate limiting por wallet address"
echo "  ✓ Nonces temporales para firmas"
echo "  ✓ Locks distribuidos"
echo ""
echo -e "${YELLOW}⚠️  Sin Redis funcionando, los usuarios NO podrán:${NC}"
echo "   - Autenticarse con sus wallets"
echo "   - Generar challenges para firmar"
echo "   - Verificar firmas"
echo "   - Mantener sesiones activas"
echo ""

