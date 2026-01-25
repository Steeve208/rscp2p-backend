# Solución de Problemas de CORS y Conexión

## Problema: Frontend no puede conectar al backend

El error muestra: "Error de red: No se puede conectar al backend"
URL: `http://64.23.151.47:3000/api`

## Soluciones

### 1. Verificar y Actualizar CORS_ORIGIN en .env

El backend necesita permitir el origen del frontend. Edita el `.env`:

```bash
cd /var/www/p2prsc-backend
nano .env
```

Busca `CORS_ORIGIN` y actualízalo:

**Opción A: Si conoces el dominio del frontend**
```env
CORS_ORIGIN=http://localhost:5173,http://localhost:3000,http://localhost:8080,https://tu-dominio-frontend.com
```

**Opción B: Permitir todos los orígenes (solo para desarrollo)**
```env
CORS_ORIGIN=*
```

**Opción C: Permitir cualquier origen localhost (desarrollo)**
```env
CORS_ORIGIN=http://localhost:*,http://127.0.0.1:*
```

Guarda y reinicia:
```bash
pm2 restart p2p-rsc-backend
```

---

### 2. Verificar que el Puerto 3000 esté Abierto

```bash
# Verificar que el servidor está escuchando
ss -tlnp | grep 3000
# O
netstat -tlnp | grep 3000

# Si no está instalado netstat
apt install net-tools -y
```

---

### 3. Configurar Firewall (UFW)

```bash
# Verificar estado del firewall
ufw status

# Si está activo, permitir puerto 3000
ufw allow 3000/tcp

# O permitir desde cualquier IP (menos seguro)
ufw allow from any to any port 3000
```

---

### 4. Verificar que el Backend Está Escuchando en 0.0.0.0

El backend debe escuchar en `0.0.0.0` (todas las interfaces), no solo `localhost`.

Verifica en `src/main.ts` línea 123:
```typescript
await app.listen(port, '0.0.0.0');
```

Si dice `localhost`, cámbialo a `0.0.0.0` y recompila.

---

### 5. Probar Conexión desde el Servidor

```bash
# Desde el servidor
curl http://localhost:3000/api/health

# Desde fuera (reemplaza con tu IP)
curl http://64.23.151.47:3000/api/health
```

---

### 6. Verificar Logs del Backend

```bash
pm2 logs p2p-rsc-backend --lines 50
```

Busca errores de CORS o conexión.

---

## Solución Rápida (Permitir Todos los Orígenes Temporalmente)

Si necesitas probar rápido, permite todos los orígenes:

```bash
cd /var/www/p2prsc-backend

# Editar .env
nano .env

# Cambiar CORS_ORIGIN a:
CORS_ORIGIN=*

# Reiniciar
pm2 restart p2p-rsc-backend

# Ver logs
pm2 logs p2p-rsc-backend --lines 20
```

**⚠️ IMPORTANTE**: `CORS_ORIGIN=*` solo para desarrollo. En producción, especifica los dominios exactos.

---

## Verificar que Funciona

Después de los cambios, prueba desde el navegador:

```javascript
// En la consola del navegador del frontend
fetch('http://64.23.151.47:3000/api/health')
  .then(r => r.json())
  .then(console.log)
  .catch(console.error);
```

Si funciona, deberías ver la respuesta del health check.

