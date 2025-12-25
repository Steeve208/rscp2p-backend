# Comandos Rápidos - Configuración de Base de Datos

## ⚠️ IMPORTANTE: Sistema P2P Wallet-to-Wallet

Este backend usa autenticación basada en **wallets** (no emails/passwords).
Los usuarios se identifican por su `wallet_address` y se autentican firmando mensajes.

## 🚀 Configuración Inicial (Una sola vez)

```bash
cd /var/www/p2prsc-backend
chmod +x setup-postgresql.sh
./setup-postgresql.sh
```

## ✅ Verificar Conexión

```bash
cd /var/www/p2prsc-backend
chmod +x verificar-db.sh
./verificar-db.sh
```

## 📦 Ejecutar Migraciones

```bash
cd /var/www/p2prsc-backend
npm run migration:run
```

## 🔄 Reiniciar Backend

```bash
pm2 restart p2p-rsc-backend
pm2 logs p2p-rsc-backend
```

## 🔍 Verificar Estado de PostgreSQL

```bash
systemctl status postgresql
```

## 🔐 Conectarse a PostgreSQL Manualmente

```bash
# Con usuario postgres
sudo -u postgres psql

# Con usuario rsc_user (después de configurar .env)
source /var/www/p2prsc-backend/.env
psql -h localhost -U rsc_user -d rsc_db
```

## 📊 Ver Tablas Creadas

```bash
source /var/www/p2prsc-backend/.env
PGPASSWORD="$DB_PASSWORD" psql -h localhost -U rsc_user -d rsc_db -c "\dt"
```

## 🐛 Ver Logs de PostgreSQL

```bash
sudo journalctl -u postgresql -n 50 -f
```

## 🔴 Verificar Redis (CRÍTICO para autenticación)

```bash
cd /var/www/p2prsc-backend
chmod +x verificar-redis.sh
./verificar-redis.sh
```

## 📊 Ver Estado de Redis

```bash
# Conectar a Redis
redis-cli -h localhost -p 6379

# Ver información
INFO server
INFO memory
INFO stats

# Ver claves de sesiones (ejemplo)
KEYS auth:session:*
KEYS auth:nonce:*
```

