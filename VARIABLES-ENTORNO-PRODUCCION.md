# Variables de Entorno para Producción

## 📝 Instrucciones

1. Crear un archivo `.env` en la raíz del proyecto
2. Copiar las variables de abajo
3. Reemplazar los valores con tus datos reales

## 🔐 Variables Requeridas

```env
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
DB_USERNAME=postgres
DB_PASSWORD=TU_PASSWORD_SEGURO_AQUI
DB_DATABASE=rsc_db

# ============================================
# REDIS
# ============================================

REDIS_HOST=localhost
REDIS_PORT=6379
REDIS_PASSWORD=TU_PASSWORD_REDIS_AQUI

# ============================================
# JWT (AUTENTICACIÓN)
# ============================================

# ⚠️ IMPORTANTE: Generar un secreto seguro de al menos 32 caracteres
# Puedes generar uno con: openssl rand -base64 32
JWT_SECRET=TU_JWT_SECRET_MUY_SEGURO_DE_AL_MENOS_32_CARACTERES_AQUI
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
```

## 🔑 Generar JWT_SECRET

### En Windows (PowerShell):
```powershell
# Generar secreto seguro
[Convert]::ToBase64String((1..32 | ForEach-Object { Get-Random -Maximum 256 }))
```

### En Linux/Mac:
```bash
openssl rand -base64 32
```

## ⚠️ Importante

1. **NUNCA** compartir el archivo `.env` con contraseñas reales
2. **NUNCA** subir `.env` al repositorio
3. En producción, usar variables de entorno del servidor o un gestor de secretos
4. `JWT_SECRET` debe ser único y seguro (mínimo 32 caracteres)
5. `CORS_ORIGIN` debe incluir todos los dominios del frontend

