# Sistema de Autenticación Web3

Sistema de autenticación basado en wallets sin usuarios tradicionales (sin emails ni passwords).

## Características

- ✅ Autenticación mediante firma de mensaje con wallet
- ✅ Sesiones temporales con JWT
- ✅ Protección anti-spam con rate limiting
- ✅ Nonces únicos con TTL
- ✅ Refresh tokens para renovar sesiones
- ✅ Sin emails ni passwords

## Flujo de Autenticación

### 1. Obtener Challenge (Nonce)

El cliente solicita un challenge para su wallet address:

```http
POST /api/auth/challenge
Content-Type: application/json

{
  "walletAddress": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb"
}
```

**Respuesta:**
```json
{
  "nonce": "0x...",
  "message": "Bienvenido a rsc.finance\n\nPor favor, firma este mensaje..."
}
```

### 2. Firmar Mensaje

El cliente debe firmar el mensaje recibido usando su wallet (MetaMask, WalletConnect, etc.):

```javascript
const signature = await signer.signMessage(message);
```

### 3. Verificar Firma

El cliente envía la firma para autenticarse:

```http
POST /api/auth/verify
Content-Type: application/json

{
  "walletAddress": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
  "nonce": "0x...",
  "signature": "0x..."
}
```

**Respuesta:**
```json
{
  "accessToken": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refreshToken": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user": {
    "id": "uuid",
    "walletAddress": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
    "createdAt": "2024-01-01T00:00:00.000Z"
  }
}
```

## Uso de Tokens

### Acceso a Rutas Protegidas

Incluir el token en el header `Authorization`:

```http
GET /api/auth/me
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

### Refrescar Token

Cuando el access token expire, usar el refresh token:

```http
POST /api/auth/refresh
Content-Type: application/json

{
  "refreshToken": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

**Respuesta:**
```json
{
  "accessToken": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

### Cerrar Sesión

```http
POST /api/auth/logout
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

## Protección Anti-Spam

El sistema implementa rate limiting para prevenir abusos:

- **Challenge**: Máximo 10 solicitudes por minuto por wallet
- **Verify**: Máximo 5 solicitudes por minuto por wallet

## Seguridad

- Los nonces expiran después de 5 minutos
- Los nonces son de un solo uso (se eliminan después de verificar)
- Las sesiones se almacenan en Redis con TTL
- Los tokens JWT tienen expiración configurable
- Validación de direcciones Ethereum
- Verificación criptográfica de firmas

## Configuración

Variables de entorno necesarias:

```env
JWT_SECRET=tu-secret-key-super-segura
JWT_EXPIRES_IN=24h
APP_DOMAIN=rsc.finance
REDIS_HOST=localhost
REDIS_PORT=6379
```

## Endpoints

| Método | Endpoint | Descripción | Autenticación |
|--------|----------|-------------|---------------|
| POST | `/api/auth/challenge` | Obtener nonce para firmar | No |
| POST | `/api/auth/verify` | Verificar firma y autenticar | No |
| POST | `/api/auth/refresh` | Refrescar access token | No |
| GET | `/api/auth/me` | Obtener perfil del usuario | Sí |
| POST | `/api/auth/logout` | Cerrar sesión | Sí |

## Ejemplo de Uso en Frontend

```typescript
// 1. Obtener challenge
const { nonce, message } = await fetch('/api/auth/challenge', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ walletAddress: userAddress }),
}).then(r => r.json());

// 2. Firmar mensaje
const signature = await signer.signMessage(message);

// 3. Verificar y obtener tokens
const { accessToken, refreshToken } = await fetch('/api/auth/verify', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    walletAddress: userAddress,
    nonce,
    signature,
  }),
}).then(r => r.json());

// 4. Guardar tokens
localStorage.setItem('accessToken', accessToken);
localStorage.setItem('refreshToken', refreshToken);

// 5. Usar en requests
const response = await fetch('/api/auth/me', {
  headers: {
    'Authorization': `Bearer ${accessToken}`,
  },
});
```
