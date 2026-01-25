# Logs en DigitalOcean – comprobar que todo funciona

En la **consola de DigitalOcean** (o por SSH) puedes usar estos comandos para revisar el estado de la app.

---

## 1. Comandos rápidos

```bash
# Ver estado de PM2
pm2 list

# Ver últimas líneas de logs (stdout + stderr)
pm2 logs p2p-rsc-backend --lines 50

# Ver solo errores
pm2 logs p2p-rsc-backend --err --lines 100

# Seguir logs en tiempo real
pm2 logs p2p-rsc-backend
```

---

## 2. Señales de que la app arrancó bien (al iniciar)

En `pm2 logs` deberías ver algo como:

| Mensaje | Significado |
|---------|-------------|
| `🚀 Starting application in production mode...` | Arranque correcto |
| `✅ Helmet security headers enabled` | Seguridad configurada |
| `✅ CORS enabled for: ...` | CORS ok |
| `✅ Global validation pipe configured` | Validación ok |
| `✅ Global exception filter configured` | Filtro de errores ok |
| `✅ Transform interceptor configured` | Interceptor ok |
| `✅ Global prefix set to: /api` | Rutas bajo /api |
| `🚀 Application is running on: http://localhost:3000/api` | Servidor escuchando |

Si ves **todos** esos mensajes, el bootstrap está correcto.

---

## 3. Recuperación de jobs (al iniciar)

Si `JobsModule` está cargado:

| Mensaje | Significado |
|---------|-------------|
| `Iniciando recuperación de jobs...` | JobRecoveryService arrancó |
| `Recuperación de jobs completada` | Locks y estado ok |

---

## 4. Jobs programados (cada X tiempo)

### CleanupJob (cada hora)

- `Iniciando limpieza de órdenes expiradas...`
- `Limpieza de órdenes expiradas completada: X canceladas, Y errores`

### ConsistencyCheckJob (cada 30 min)

- `Iniciando verificación de inconsistencias...`
- Si todo bien: `No se encontraron inconsistencias`
- Si hay problemas: `Inconsistencias detectadas: N` y luego las líneas `- Order xxx: ...`

---

## 5. Probar que la API responde (desde el servidor)

```bash
# Health (si HealthModule está en la app)
curl -s http://localhost:3000/api/health | head -20

# Si no hay /health, probar cualquier GET (ej. órdenes con auth, o una ruta que exista)
curl -s -o /dev/null -w "%{http_code}" http://localhost:3000/api/
# 404 es normal si no hay ruta en /api; lo importante es que no sea 502/503
```

Desde **fuera** (tu PC), sustituye `localhost` por la IP o dominio del Droplet:

```bash
curl -s https://TU_DOMINIO_O_IP/api/health
```

---

## 6. Qué indica que algo va mal

| En logs | Posible causa |
|---------|----------------|
| `ERROR` en muchas líneas | Excepciones o fallos en servicios |
| `Cannot POST /` o `404` en `POST /` | Algo hace POST a la raíz; no es crítico si el resto funciona |
| `Relation with property path ... was not found` | Error de TypeORM (relación mal definida o query incorrecta) |
| `ECONNREFUSED` / `connect ECONNREFUSED` | PostgreSQL o Redis inaccesible; revisar `.env` y que los servicios estén up |
| `EADDRINUSE` | Puerto en uso; otro proceso o doble arranque |
| `❌ Failed to start application` | Error en el bootstrap; mirar la línea siguiente (stack) |
| `Exited with code 1` en PM2 | La app se cae al arrancar o en runtime |

---

## 7. Revisar que el proceso y el puerto están bien

```bash
# Proceso Node
ps aux | grep "dist/main.js"

# Qué está escuchando en el puerto (ej. 3000)
sudo lsof -i :3000
# o
sudo ss -tlnp | grep 3000
```

---

## 8. Resumen rápido “¿Va bien?”

1. `pm2 list` → `p2p-rsc-backend` en **online**.
2. `pm2 logs p2p-rsc-backend --lines 30` → mensajes de bootstrap con ✅ y `Application is running`.
3. `curl http://localhost:3000/api/health` (o `GET /api/orders` si no tienes health) devuelve 200 o 404 en una ruta existente, no 502/503.
4. No hay cascada de `ERROR` ni `Exited with code 1` en los logs.

Si se cumple eso, lo normal es que la app en DigitalOcean esté funcionando correctamente.
