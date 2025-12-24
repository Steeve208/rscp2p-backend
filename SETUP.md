# Guía de Configuración - RSC Backend

## Estado Actual

✅ **Completado:**
- Estructura del proyecto creada
- Archivo `.env` creado
- `package.json` con todas las dependencias

❌ **Pendiente:**
- Instalación de dependencias npm
- Instalación y configuración de PostgreSQL
- Instalación y configuración de Redis

## Pasos para Completar la Configuración

### 1. Instalar Dependencias NPM

```bash
npm install
```

Esto instalará todas las dependencias del proyecto (puede tardar varios minutos).

### 2. Instalar PostgreSQL

**Opción A: Instalación Manual (Recomendado para Windows)**

1. Descargar PostgreSQL desde: https://www.postgresql.org/download/windows/
2. Instalar con las opciones por defecto
3. Recordar la contraseña del usuario `postgres`
4. Actualizar el archivo `.env` con la contraseña correcta

**Opción B: Usar Docker (si tienes Docker Desktop)**

```bash
docker run --name postgres-rsc -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=rsc_db -p 5432:5432 -d postgres:14
```

### 3. Instalar Redis

**Opción A: Instalación Manual (Windows)**

1. Descargar Redis para Windows desde: https://github.com/microsoftarchive/redis/releases
2. O usar WSL2 con Redis
3. O usar Memurai (Redis para Windows): https://www.memurai.com/

**Opción B: Usar Docker**

```bash
docker run --name redis-rsc -p 6379:6379 -d redis:7-alpine
```

### 4. Crear la Base de Datos

Una vez PostgreSQL esté instalado y corriendo:

```bash
# Conectar a PostgreSQL
psql -U postgres

# Crear la base de datos
CREATE DATABASE rsc_db;

# Salir
\q
```

### 5. Ejecutar Migraciones

```bash
npm run migration:run
```

### 6. Iniciar el Backend

```bash
npm run dev
```

El servidor debería iniciar en: `http://localhost:3000/api`

## Verificación Rápida

```bash
# Verificar PostgreSQL
psql -U postgres -c "SELECT version();"

# Verificar Redis
redis-cli ping

# Verificar Node.js
node --version

# Verificar npm
npm --version
```

## Solución de Problemas

### Error: "Cannot connect to database"
- Verificar que PostgreSQL esté corriendo
- Verificar credenciales en `.env`
- Verificar que la base de datos `rsc_db` exista

### Error: "Cannot connect to Redis"
- Verificar que Redis esté corriendo
- Verificar puerto 6379 disponible

### Error: "Module not found"
- Ejecutar `npm install` nuevamente
- Eliminar `node_modules` y `package-lock.json`, luego `npm install`


