# 🚀 API Quick Reference - RSC Finance

## Base URL
```
http://64.23.151.47:3000/api
```

---

## 🔐 Autenticación (Wallet-Based)

### Flujo de Autenticación

```javascript
// 1. Solicitar challenge
POST /api/auth/challenge
Body: { walletAddress: "0x..." }
→ { nonce, message }

// 2. Firmar mensaje con wallet
const signature = await wallet.signMessage(message);

// 3. Verificar y obtener tokens
POST /api/auth/verify
Body: { walletAddress, nonce, signature }
→ { accessToken, refreshToken, user }

// 4. Usar token en requests
Headers: { Authorization: "Bearer {accessToken}" }
```

---

## 📋 Endpoints Principales

### 👤 Usuarios
```
GET  /api/users                    # Listar usuarios (público)
GET  /api/users/:id                # Usuario por ID (público)
GET  /api/users/wallet/:address    # Usuario por wallet (público)
GET  /api/users/ranking            # Ranking (público)
GET  /api/users/stats/:address     # Estadísticas (público)
GET  /api/users/me/profile         # Mi perfil (requiere auth)
```

### 💼 Órdenes P2P
```
POST /api/orders                   # Crear orden (requiere auth)
GET  /api/orders                   # Listar órdenes (público)
GET  /api/orders/:id               # Orden por ID (público)
GET  /api/orders/:id/status        # Estado de orden (público)
PUT  /api/orders/:id/accept        # Aceptar orden (requiere auth)
PUT  /api/orders/:id/cancel        # Cancelar orden (requiere auth)
PUT  /api/orders/:id/complete      # Completar orden (requiere auth)
PUT  /api/orders/:id/dispute       # Marcar como disputada (requiere auth)
GET  /api/orders/me                # Mis órdenes (requiere auth)
```

### 🔒 Escrow
```
POST /api/escrow                   # Crear mapeo order ↔ escrow
GET  /api/escrow/:id               # Escrow por ID
GET  /api/escrow/order/:orderId    # Escrow por order ID
GET  /api/escrow/blockchain/:escrowId  # Escrow por escrow ID
GET  /api/escrow/mapping            # Obtener mapeo
GET  /api/escrow/validate/:orderId  # Validar consistencia
PUT  /api/escrow/:escrowId         # Actualizar escrow
```

### ⚖️ Disputas
```
POST /api/disputes                 # Crear disputa (requiere auth)
GET  /api/disputes                 # Listar disputas
GET  /api/disputes/:id             # Disputa por ID
POST /api/disputes/:id/evidence    # Agregar evidencia (requiere auth)
PUT  /api/disputes/:id/resolve    # Resolver disputa (requiere auth)
PUT  /api/disputes/:id/close      # Cerrar disputa
PUT  /api/disputes/:id/escalate   # Escalar disputa (requiere auth)
GET  /api/disputes/expiring        # Disputas próximas a expirar
```

### ⭐ Reputación
```
GET  /api/reputation/:userId       # Reputación de usuario
GET  /api/reputation/:userId/history  # Historial
GET  /api/reputation/:userId/stats # Estadísticas
POST /api/reputation/:userId/recalculate  # Recalcular
GET  /api/reputation/ranking       # Ranking
POST /api/reputation/penalty      # Aplicar penalización
POST /api/reputation/bonus        # Aplicar bonus
```

### 🔔 Notificaciones
```
GET  /api/notifications             # Mis notificaciones (requiere auth)
GET  /api/notifications/unread-count  # Contador no leídas (requiere auth)
PUT  /api/notifications/:id/read   # Marcar como leída (requiere auth)
PUT  /api/notifications/read-all   # Marcar todas como leídas (requiere auth)
```

### ❤️ Health
```
GET  /api/health                   # Health general
GET  /api/health/live              # Liveness
GET  /api/health/ready             # Readiness
```

---

## 📊 Estados Importantes

### Order Status
- `CREATED` → Orden creada, esperando comprador
- `AWAITING_FUNDS` → Comprador aceptó, esperando fondos
- `ONCHAIN_LOCKED` → Fondos bloqueados en escrow
- `COMPLETED` → Orden completada
- `REFUNDED` → Fondos devueltos
- `DISPUTED` → Orden en disputa

### Escrow Status
- `PENDING` → Pendiente
- `LOCKED` → Fondos bloqueados
- `RELEASED` → Fondos liberados
- `REFUNDED` → Fondos devueltos
- `DISPUTED` → En disputa

### Dispute Status
- `OPEN` → Abierta
- `IN_REVIEW` → En revisión
- `RESOLVED` → Resuelta
- `CLOSED` → Cerrada
- `ESCALATED` → Escalada

---

## 📡 WebSockets

### Conexión
```javascript
const socket = io('http://64.23.151.47:3000', {
  auth: { token: accessToken }
});
```

### Eventos Market
```javascript
socket.emit('subscribe');
socket.on('order:created', (data) => { ... });
socket.on('order:updated', (data) => { ... });
socket.on('order:accepted', (data) => { ... });
```

### Eventos Notificaciones
```javascript
socket.emit('subscribe:user', { walletAddress: '0x...' });
socket.on('notification', (data) => { ... });
```

---

## 🔑 Headers Requeridos

```javascript
{
  'Content-Type': 'application/json',
  'Authorization': 'Bearer {accessToken}'  // Para endpoints protegidos
}
```

---

## ⚡ Rate Limits

- Auth Challenge: **10/min** por wallet
- Auth Verify: **5/min** por wallet
- Crear Orden: **10/min**
- Aceptar Orden: **5/min**
- Listar Órdenes: **30/min**
- General: **100/min** por IP

---

## 📝 Ejemplo Completo

```javascript
// 1. Autenticarse
const { nonce, message } = await fetch('/api/auth/challenge', {
  method: 'POST',
  body: JSON.stringify({ walletAddress: '0x...' })
}).then(r => r.json());

const signature = await wallet.signMessage(message);

const { accessToken } = await fetch('/api/auth/verify', {
  method: 'POST',
  body: JSON.stringify({ walletAddress: '0x...', nonce, signature })
}).then(r => r.json());

// 2. Crear orden
const order = await fetch('/api/orders', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${accessToken}`,
    'Content-Type': 'application/json'
  },
  body: JSON.stringify({
    cryptoCurrency: 'ETH',
    cryptoAmount: '0.5',
    fiatCurrency: 'USD',
    fiatAmount: '1500.00',
    pricePerUnit: '3000.00',
    paymentMethod: 'BANK_TRANSFER'
  })
}).then(r => r.json());

// 3. Listar órdenes
const orders = await fetch('/api/orders?status=CREATED&cryptoCurrency=ETH')
  .then(r => r.json());
```

---

## 🗄️ Estructura de Datos Clave

### User
```typescript
{
  id: string;
  walletAddress: string;  // Único identificador
  reputationScore: number;
  isActive: boolean;
  createdAt: Date;
}
```

### Order
```typescript
{
  id: string;
  sellerId: string;
  buyerId: string | null;
  cryptoCurrency: string;
  cryptoAmount: string;
  fiatCurrency: string;
  fiatAmount: string;
  status: OrderStatus;
  escrowId: string | null;
  createdAt: Date;
}
```

---

## 🚨 Códigos HTTP

- `200` OK
- `201` Created
- `400` Bad Request
- `401` Unauthorized
- `404` Not Found
- `429` Too Many Requests
- `500` Server Error

---

**Documentación completa**: Ver `API-DOCUMENTATION-FRONTEND.md`

