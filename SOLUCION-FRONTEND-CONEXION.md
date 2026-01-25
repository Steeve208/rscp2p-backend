# Solución: Frontend No Puede Conectar al Backend

## ✅ Backend Funciona Correctamente

El backend está respondiendo correctamente:
- ✅ Puerto 3000 abierto
- ✅ CORS configurado como `*` (todos los orígenes)
- ✅ Backend escuchando en `0.0.0.0:3000`
- ✅ Health check responde desde fuera del servidor

## 🔍 Problema en el Frontend

El error en el frontend dice: "Error de red: No se puede conectar al backend"
URL: `http://64.23.151.47:3000/api`

## Soluciones para el Frontend

### 1. Verificar URL en el Frontend

Asegúrate de que el frontend esté usando la URL correcta:

```javascript
// En tu archivo de configuración del frontend
const API_URL = 'http://64.23.151.47:3000/api';
// O si tienes variable de entorno:
const API_URL = import.meta.env.VITE_API_URL || 'http://64.23.151.47:3000/api';
```

### 2. Verificar CORS en el Navegador

Abre la consola del navegador (F12) y ejecuta:

```javascript
fetch('http://64.23.151.47:3000/api/health', {
  method: 'GET',
  headers: {
    'Content-Type': 'application/json'
  }
})
.then(r => r.json())
.then(data => console.log('✅ OK:', data))
.catch(err => console.error('❌ Error:', err));
```

**Si ves un error de CORS**, el problema es que el navegador está bloqueando. Verifica:
- Que el backend tenga `CORS_ORIGIN=*` en el `.env`
- Que el backend esté reiniciado después del cambio

**Si ves un error de red**, verifica:
- Que no haya un proxy configurado
- Que no haya un firewall del navegador
- Que la URL sea exactamente `http://64.23.151.47:3000/api`

### 3. Verificar Configuración del Cliente HTTP

Si usas axios, fetch, o similar, verifica:

```javascript
// Ejemplo con fetch
const response = await fetch('http://64.23.151.47:3000/api/health', {
  method: 'GET',
  headers: {
    'Content-Type': 'application/json',
  },
  mode: 'cors', // Importante para CORS
  credentials: 'include' // Si usas cookies
});
```

### 4. Verificar Variables de Entorno del Frontend

Si el frontend usa variables de entorno, verifica:

```env
# .env del frontend
VITE_API_URL=http://64.23.151.47:3000/api
# O
REACT_APP_API_URL=http://64.23.151.47:3000/api
# Dependiendo del framework
```

### 5. Probar con curl desde el Navegador

Abre la consola del navegador y ejecuta:

```javascript
// Prueba directa
fetch('http://64.23.151.47:3000/api/health')
  .then(r => r.json())
  .then(console.log)
  .catch(console.error);
```

### 6. Verificar Network Tab

1. Abre DevTools (F12)
2. Ve a la pestaña "Network"
3. Intenta hacer una petición desde el frontend
4. Revisa la petición fallida:
   - ¿Qué URL está usando?
   - ¿Qué error muestra?
   - ¿Hay headers de CORS en la respuesta?

### 7. Posibles Problemas Específicos

#### Problema: Mixed Content (HTTP/HTTPS)

Si el frontend está en HTTPS y el backend en HTTP, el navegador bloqueará:

**Solución**: Usar HTTPS para el backend o configurar el frontend para permitir HTTP en desarrollo.

#### Problema: Preflight OPTIONS Falla

El navegador envía una petición OPTIONS antes de la petición real. Verifica que el backend responda correctamente:

```bash
# Probar OPTIONS desde el servidor
curl -X OPTIONS http://localhost:3000/api/health \
  -H "Origin: http://localhost:5173" \
  -H "Access-Control-Request-Method: GET" \
  -v
```

Debería responder con headers CORS.

---

## 🔧 Comandos para Verificar en el Servidor

```bash
# Verificar que CORS está configurado como *
grep CORS_ORIGIN /var/www/p2prsc-backend/.env

# Ver logs del backend para ver peticiones
pm2 logs p2p-rsc-backend --lines 50 | grep -i cors

# Verificar que el backend está escuchando
ss -tlnp | grep 3000
```

---

## 📝 Configuración Recomendada para Producción

Para producción, configura:

```env
# .env del backend
CORS_ORIGIN=https://tu-dominio-frontend.com,https://www.tu-dominio-frontend.com
```

Y usa Nginx como proxy reverso en el puerto 80/443.

---

## 🚀 Prueba Rápida

Ejecuta esto en la consola del navegador del frontend:

```javascript
// Test completo
async function testBackend() {
  try {
    console.log('🔍 Probando conexión...');
    const response = await fetch('http://64.23.151.47:3000/api/health');
    console.log('✅ Status:', response.status);
    const data = await response.json();
    console.log('✅ Datos:', data);
    return true;
  } catch (error) {
    console.error('❌ Error:', error);
    console.error('Tipo:', error.name);
    console.error('Mensaje:', error.message);
    return false;
  }
}

testBackend();
```

Si esto funciona pero el frontend no, el problema está en la configuración del cliente HTTP del frontend.

