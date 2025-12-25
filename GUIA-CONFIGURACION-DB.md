# Guía de Configuración de Base de Datos - Digital Ocean

Esta guía te ayudará a configurar PostgreSQL en tu servidor de Digital Ocean para el backend de RSC Finance.

## 🎯 Sobre RSC Finance

RSC Finance es una plataforma **P2P wallet-to-wallet** donde:
- ✅ Los usuarios se autentican con sus **wallets** (MetaMask, WalletConnect, etc.)
- ✅ No hay emails ni passwords tradicionales
- ✅ Autenticación mediante **firma de mensajes** criptográficos
- ✅ Transacciones **peer-to-peer** directas entre wallets
- ✅ Sistema de **reputación off-chain** basado en wallet addresses

### Arquitectura de Datos

- **PostgreSQL**: Almacena usuarios (por wallet_address), órdenes P2P, escrows, disputas, reputación
- **Redis**: Sesiones JWT, rate limiting, nonces temporales, locks distribuidos
- **Blockchain**: Escucha eventos on-chain y reconcilia estados

## 📋 Requisitos Previos

- Servidor Ubuntu/Debian en Digital Ocean
- Acceso SSH al servidor como root
- PostgreSQL instalado (ya lo tienes instalado)
- Redis instalado (para sesiones JWT y rate limiting)

## 🚀 Pasos de Configuración

### 1. Subir el script al servidor

Si estás trabajando desde tu máquina local, sube el script al servidor:

```bash
# Desde tu máquina local
scp setup-postgresql.sh root@tu-servidor:/var/www/p2prsc-backend/
```

O si ya estás en el servidor, el script debería estar en `/var/www/p2prsc-backend/`

### 2. Ejecutar el script de configuración

```bash
cd /var/www/p2prsc-backend
chmod +x setup-postgresql.sh
./setup-postgresql.sh
```

El script realizará automáticamente:
- ✅ Verificación de PostgreSQL
- ✅ Configuración de conexiones locales
- ✅ Creación de usuario `rsc_user`
- ✅ Creación de base de datos `rsc_db`
- ✅ Otorgamiento de permisos
- ✅ Creación/actualización del archivo `.env`

### 3. Guardar las credenciales

**⚠️ IMPORTANTE**: El script mostrará una contraseña generada. **Guárdala de forma segura**.

Ejemplo de output:
```
✓ Contraseña generada: aB3xY9mK2pQ7vN5tR8wL4jH6
```

### 4. Verificar la conexión

```bash
cd /var/www/p2prsc-backend
chmod +x verificar-db.sh
./verificar-db.sh
```

Este script verificará que la conexión funcione correctamente.

### 5. Ejecutar las migraciones

```bash
cd /var/www/p2prsc-backend
npm run migration:run
```

Esto creará todas las tablas necesarias en la base de datos.

### 6. Reiniciar el backend

```bash
pm2 restart p2p-rsc-backend
pm2 logs p2p-rsc-backend
```

Verifica que no haya errores de conexión en los logs.

## 🔧 Configuración Manual (Si el script falla)

### Crear usuario y base de datos manualmente

```bash
# Conectarse como usuario postgres
sudo -u postgres psql

# En la consola de PostgreSQL:
CREATE USER rsc_user WITH PASSWORD 'tu_contraseña_segura_aqui';
CREATE DATABASE rsc_db OWNER rsc_user;
GRANT ALL PRIVILEGES ON DATABASE rsc_db TO rsc_user;
\q
```

### Configurar PostgreSQL para conexiones locales

Editar `/etc/postgresql/14/main/pg_hba.conf`:

```bash
sudo nano /etc/postgresql/14/main/pg_hba.conf
```

Asegúrate de tener estas líneas (después de `# IPv4 local connections:`):

```
local   all             all                                     md5
host    all             all             127.0.0.1/32            md5
```

Editar `/etc/postgresql/14/main/postgresql.conf`:

```bash
sudo nano /etc/postgresql/14/main/postgresql.conf
```

Asegúrate de que tenga:

```
listen_addresses = 'localhost'
```

Reiniciar PostgreSQL:

```bash
sudo systemctl restart postgresql
```

### Configurar archivo .env

Editar `/var/www/p2prsc-backend/.env`:

```env
DB_HOST=localhost
DB_PORT=5432
DB_USERNAME=rsc_user
DB_PASSWORD=tu_contraseña_aqui
DB_DATABASE=rsc_db
```

## 🐛 Solución de Problemas

### Error: "Connection refused"

**Causa**: PostgreSQL no está escuchando en localhost

**Solución**:
```bash
# Verificar que PostgreSQL esté corriendo
systemctl status postgresql

# Si no está corriendo
systemctl start postgresql
systemctl enable postgresql

# Verificar configuración
sudo -u postgres psql -c "SHOW listen_addresses;"
```

### Error: "password authentication failed"

**Causa**: Contraseña incorrecta o usuario no existe

**Solución**:
```bash
# Verificar usuario
sudo -u postgres psql -c "\du"

# Cambiar contraseña del usuario
sudo -u postgres psql -c "ALTER USER rsc_user WITH PASSWORD 'nueva_contraseña';"
```

### Error: "database does not exist"

**Causa**: La base de datos no fue creada

**Solución**:
```bash
sudo -u postgres psql -c "CREATE DATABASE rsc_db OWNER rsc_user;"
```

### Error: "permission denied"

**Causa**: El usuario no tiene permisos

**Solución**:
```bash
sudo -u postgres psql -d rsc_db -c "GRANT ALL PRIVILEGES ON DATABASE rsc_db TO rsc_user;"
sudo -u postgres psql -d rsc_db -c "GRANT ALL ON SCHEMA public TO rsc_user;"
```

## 📝 Verificación Final

Después de la configuración, verifica que todo funcione:

```bash
# 1. Verificar conexión
./verificar-db.sh

# 2. Verificar que el backend se conecte
pm2 logs p2p-rsc-backend --lines 50

# 3. Verificar tablas creadas
PGPASSWORD="tu_contraseña" psql -h localhost -U rsc_user -d rsc_db -c "\dt"
```

## 🔒 Seguridad

1. **Nunca** compartas el archivo `.env` con las contraseñas
2. **Nunca** subas `.env` al repositorio Git
3. Usa contraseñas seguras (mínimo 20 caracteres)
4. Considera usar un firewall para limitar acceso a PostgreSQL
5. En producción, considera usar SSL para las conexiones
6. **Redis es crítico**: Sin Redis, los usuarios no podrán autenticarse (sesiones JWT y rate limiting)

## 💡 Notas sobre el Sistema P2P Wallet-to-Wallet

### Autenticación
- Los usuarios solicitan un **challenge** (nonce) para su wallet address
- Firman el mensaje con su wallet privada
- El backend verifica la firma y emite tokens JWT
- **No hay passwords**: Solo firmas criptográficas

### Base de Datos
- Cada usuario se identifica por su `wallet_address` (único)
- No se almacena información personal
- Sistema pseudónimo: solo wallet addresses y reputación

### Redis
- **Sesiones JWT**: Refresh tokens para mantener sesiones activas
- **Rate Limiting**: Previene spam en autenticación (10 challenges/min, 5 verificaciones/min por wallet)
- **Nonces temporales**: Challenges firmables con TTL de 5 minutos
- **Locks distribuidos**: Para operaciones críticas (escrows, disputas)

## 📞 Soporte

Si tienes problemas, verifica:
- Logs de PostgreSQL: `sudo journalctl -u postgresql -n 50`
- Logs del backend: `pm2 logs p2p-rsc-backend`
- Estado de PostgreSQL: `systemctl status postgresql`

