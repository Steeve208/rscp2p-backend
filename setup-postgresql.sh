#!/bin/bash

# ============================================
# Script de Configuración de PostgreSQL
# para Digital Ocean - RSC Finance Backend
# ============================================

set -e  # Salir si hay algún error

echo "============================================"
echo "Configuración de PostgreSQL para RSC Finance"
echo "Plataforma P2P Wallet-to-Wallet"
echo "============================================"
echo ""
echo "ℹ️  Este sistema usa autenticación basada en wallets"
echo "   (sin emails/passwords tradicionales)"
echo ""

# Colores para output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Variables de configuración
DB_NAME="rsc_db"
DB_USER="rsc_user"
DB_PASSWORD=""
POSTGRES_USER="postgres"

# Función para generar contraseña segura
generate_password() {
    openssl rand -base64 32 | tr -d "=+/" | cut -c1-25
}

# Verificar si PostgreSQL está corriendo
echo -e "${YELLOW}[1/6]${NC} Verificando estado de PostgreSQL..."
if ! systemctl is-active --quiet postgresql; then
    echo -e "${RED}ERROR: PostgreSQL no está corriendo${NC}"
    echo "Iniciando PostgreSQL..."
    systemctl start postgresql
    systemctl enable postgresql
fi
echo -e "${GREEN}✓ PostgreSQL está corriendo${NC}"
echo ""

# Solicitar contraseña para el usuario de la base de datos
echo -e "${YELLOW}[2/6]${NC} Configurando credenciales..."
if [ -z "$DB_PASSWORD" ]; then
    echo "Generando contraseña segura para el usuario de la base de datos..."
    DB_PASSWORD=$(generate_password)
    echo -e "${GREEN}✓ Contraseña generada: ${DB_PASSWORD}${NC}"
    echo ""
    echo -e "${YELLOW}⚠️  IMPORTANTE: Guarda esta contraseña, la necesitarás para el archivo .env${NC}"
    echo ""
fi

# Configurar PostgreSQL para aceptar conexiones locales
echo -e "${YELLOW}[3/6]${NC} Configurando PostgreSQL para aceptar conexiones locales..."

# Backup del archivo de configuración
PG_VERSION=$(psql --version | grep -oP '\d+' | head -1)
PG_HBA_FILE="/etc/postgresql/${PG_VERSION}/main/pg_hba.conf"
PG_CONF_FILE="/etc/postgresql/${PG_VERSION}/main/postgresql.conf"

if [ -f "$PG_HBA_FILE" ]; then
    # Crear backup
    cp "$PG_HBA_FILE" "${PG_HBA_FILE}.backup.$(date +%Y%m%d_%H%M%S)"
    
    # Verificar si ya existe la configuración
    if ! grep -q "local.*all.*all.*md5" "$PG_HBA_FILE"; then
        # Agregar configuración para conexiones locales con md5
        sed -i '/^# IPv4 local connections:/a local   all             all                                     md5' "$PG_HBA_FILE"
    fi
    
    # Asegurar que localhost esté configurado
    if ! grep -q "^host.*all.*all.*127.0.0.1/32.*md5" "$PG_HBA_FILE"; then
        sed -i '/^# IPv4 local connections:/a host    all             all             127.0.0.1/32            md5' "$PG_HBA_FILE"
    fi
    
    echo -e "${GREEN}✓ Configuración de autenticación actualizada${NC}"
else
    echo -e "${YELLOW}⚠️  No se encontró pg_hba.conf en la ubicación esperada${NC}"
    echo "   PostgreSQL puede estar usando una configuración diferente"
fi

# Configurar postgresql.conf para escuchar en localhost
if [ -f "$PG_CONF_FILE" ]; then
    # Habilitar escucha en localhost
    sed -i "s/#listen_addresses = 'localhost'/listen_addresses = 'localhost'/" "$PG_CONF_FILE"
    sed -i "s/listen_addresses = '\*'/listen_addresses = 'localhost'/" "$PG_CONF_FILE"
    
    echo -e "${GREEN}✓ Configuración de red actualizada${NC}"
fi

# Reiniciar PostgreSQL para aplicar cambios
echo "Reiniciando PostgreSQL para aplicar cambios..."
systemctl restart postgresql
sleep 2

echo -e "${GREEN}✓ PostgreSQL reiniciado${NC}"
echo ""

# Crear usuario y base de datos
echo -e "${YELLOW}[4/6]${NC} Creando usuario y base de datos..."

# Crear usuario si no existe
sudo -u postgres psql -c "SELECT 1 FROM pg_user WHERE usename='$DB_USER'" | grep -q 1 || {
    sudo -u postgres psql -c "CREATE USER $DB_USER WITH PASSWORD '$DB_PASSWORD';"
    echo -e "${GREEN}✓ Usuario $DB_USER creado${NC}"
}

# Crear base de datos si no existe
sudo -u postgres psql -c "SELECT 1 FROM pg_database WHERE datname='$DB_NAME'" | grep -q 1 || {
    sudo -u postgres psql -c "CREATE DATABASE $DB_NAME OWNER $DB_USER;"
    echo -e "${GREEN}✓ Base de datos $DB_NAME creada${NC}"
}

# Otorgar todos los privilegios
sudo -u postgres psql -d "$DB_NAME" -c "GRANT ALL PRIVILEGES ON DATABASE $DB_NAME TO $DB_USER;"
sudo -u postgres psql -d "$DB_NAME" -c "GRANT ALL ON SCHEMA public TO $DB_USER;"
sudo -u postgres psql -d "$DB_NAME" -c "ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO $DB_USER;"
sudo -u postgres psql -d "$DB_NAME" -c "ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO $DB_USER;"

echo -e "${GREEN}✓ Permisos otorgados${NC}"
echo ""

# Verificar conexión
echo -e "${YELLOW}[5/6]${NC} Verificando conexión a la base de datos..."
if PGPASSWORD="$DB_PASSWORD" psql -h localhost -U "$DB_USER" -d "$DB_NAME" -c "SELECT version();" > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Conexión exitosa${NC}"
else
    echo -e "${RED}✗ Error al conectar a la base de datos${NC}"
    echo "   Verifica las credenciales y la configuración de PostgreSQL"
    exit 1
fi
echo ""

# Crear archivo .env de ejemplo si no existe
echo -e "${YELLOW}[6/6]${NC} Configurando variables de entorno..."

ENV_FILE="/var/www/p2prsc-backend/.env"
ENV_EXAMPLE="/var/www/p2prsc-backend/.env.example"

if [ ! -f "$ENV_FILE" ]; then
    echo "Creando archivo .env..."
    cat > "$ENV_FILE" << EOF
# ============================================
# CONFIGURACIÓN DE PRODUCCIÓN
# ============================================

# Entorno
NODE_ENV=production

# Puerto del servidor
PORT=3000

# CORS - IMPORTANTE: Configurar con el dominio del frontend
CORS_ORIGIN=https://tu-frontend.com,https://www.tu-frontend.com

# Dominio de la aplicación
APP_DOMAIN=tu-dominio.com

# ============================================
# BASE DE DATOS (PostgreSQL)
# ============================================

DB_HOST=localhost
DB_PORT=5432
DB_USERNAME=$DB_USER
DB_PASSWORD=$DB_PASSWORD
DB_DATABASE=$DB_NAME

# ============================================
# REDIS
# ============================================

REDIS_HOST=localhost
REDIS_PORT=6379
REDIS_PASSWORD=

# ============================================
# JWT (AUTENTICACIÓN)
# ============================================

# ⚠️ IMPORTANTE: Generar un secreto seguro de al menos 32 caracteres
# Puedes generar uno con: openssl rand -base64 32
JWT_SECRET=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-40)
JWT_EXPIRES_IN=24h
JWT_REFRESH_EXPIRES_IN=7d

# ============================================
# RATE LIMITING
# ============================================

RATE_LIMIT_TTL=60
RATE_LIMIT_MAX=100

# ============================================
# BLOCKCHAIN (OPCIONAL - Para integración futura)
# ============================================

# Dejar vacío si no se usa blockchain
BLOCKCHAIN_RPC_URL=
BLOCKCHAIN_NETWORK=mainnet
BLOCKCHAIN_PRIVATE_KEY=
ESCROW_CONTRACT_ADDRESS=
EOF
    chmod 600 "$ENV_FILE"
    echo -e "${GREEN}✓ Archivo .env creado en $ENV_FILE${NC}"
    echo ""
    echo -e "${YELLOW}⚠️  IMPORTANTE:${NC}"
    echo "   1. Revisa y actualiza las variables en $ENV_FILE"
    echo "   2. Especialmente CORS_ORIGIN y APP_DOMAIN"
    echo "   3. Configura REDIS si lo necesitas"
else
    echo -e "${YELLOW}⚠️  El archivo .env ya existe${NC}"
    echo "   Actualiza manualmente las siguientes variables:"
    echo "   DB_USERNAME=$DB_USER"
    echo "   DB_PASSWORD=$DB_PASSWORD"
    echo "   DB_DATABASE=$DB_NAME"
fi
echo ""

# Resumen final
echo "============================================"
echo -e "${GREEN}✓ Configuración completada exitosamente${NC}"
echo "============================================"
echo ""
echo "Resumen de la configuración:"
echo "  Base de datos: $DB_NAME"
echo "  Usuario: $DB_USER"
echo "  Contraseña: $DB_PASSWORD"
echo "  Host: localhost"
echo "  Puerto: 5432"
echo ""
echo "Próximos pasos:"
echo "  1. Revisa el archivo .env en /var/www/p2prsc-backend/.env"
echo "  2. ⚠️  CONFIGURA REDIS (crítico para autenticación wallet-based)"
echo "  3. Ejecuta las migraciones: npm run migration:run"
echo "  4. Reinicia el backend: pm2 restart p2p-rsc-backend"
echo ""
echo -e "${YELLOW}⚠️  IMPORTANTE:${NC}"
echo "   - Guarda la contraseña de la base de datos de forma segura"
echo "   - Redis es CRÍTICO: sin Redis, los usuarios no podrán autenticarse"
echo "   - Los usuarios se identifican por wallet_address (no emails)"
echo ""

