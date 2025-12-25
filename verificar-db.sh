#!/bin/bash

# ============================================
# Script de Verificación de Base de Datos
# ============================================

set -e

# Colores
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "============================================"
echo "Verificación de Conexión a PostgreSQL"
echo "============================================"
echo ""

# Cargar variables de entorno
if [ -f "/var/www/p2prsc-backend/.env" ]; then
    export $(cat /var/www/p2prsc-backend/.env | grep -v '^#' | xargs)
else
    echo -e "${RED}✗ No se encontró el archivo .env${NC}"
    exit 1
fi

# Verificar variables
if [ -z "$DB_HOST" ] || [ -z "$DB_USERNAME" ] || [ -z "$DB_PASSWORD" ] || [ -z "$DB_DATABASE" ]; then
    echo -e "${RED}✗ Variables de base de datos no configuradas en .env${NC}"
    exit 1
fi

echo "Intentando conectar a:"
echo "  Host: $DB_HOST"
echo "  Puerto: ${DB_PORT:-5432}"
echo "  Base de datos: $DB_DATABASE"
echo "  Usuario: $DB_USERNAME"
echo ""

# Verificar conexión
if PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "${DB_PORT:-5432}" -U "$DB_USERNAME" -d "$DB_DATABASE" -c "SELECT version();" > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Conexión exitosa${NC}"
    echo ""
    
    # Mostrar información de la base de datos
    echo "Información de la base de datos:"
    PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "${DB_PORT:-5432}" -U "$DB_USERNAME" -d "$DB_DATABASE" -c "SELECT version();" | head -3
    echo ""
    
    # Contar tablas
    TABLE_COUNT=$(PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "${DB_PORT:-5432}" -U "$DB_USERNAME" -d "$DB_DATABASE" -t -c "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public';" | tr -d ' ')
    
    if [ "$TABLE_COUNT" -eq "0" ]; then
        echo -e "${YELLOW}⚠️  No hay tablas en la base de datos${NC}"
        echo "   Ejecuta las migraciones con: npm run migration:run"
    else
        echo -e "${GREEN}✓ Base de datos tiene $TABLE_COUNT tabla(s)${NC}"
    fi
    
    echo ""
    echo -e "${GREEN}✓ Verificación completada${NC}"
else
    echo -e "${RED}✗ Error al conectar a la base de datos${NC}"
    echo ""
    echo "Posibles causas:"
    echo "  1. PostgreSQL no está corriendo: systemctl status postgresql"
    echo "  2. Credenciales incorrectas en .env"
    echo "  3. Base de datos no existe"
    echo "  4. Usuario no tiene permisos"
    exit 1
fi

