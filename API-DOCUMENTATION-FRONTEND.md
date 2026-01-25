# 📚 Documentación de API - RSC Finance Backend

## 🎯 Información General

**Base URL**: `http://64.23.151.47:3000/api` (o tu dominio configurado)

**Autenticación**: Wallet-based (sin emails/passwords)
- Los usuarios se autentican firmando mensajes con sus wallets
- Sistema P2P wallet-to-wallet

**Formato de Respuesta**:
```json
{
  "statusCode": 200,
  "data": { ... },
  "timestamp": "2025-12-25T21:50:54.296Z"
}
```

---

## 🔐 Autenticación Wallet-Based

### 1. Solicitar Challenge (Nonce)

**POST** `/api/auth/challenge`

**Body**:
```json
{
  "walletAddress": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb"
}
```

**Response**:
```json
{
  "nonce": "0x...",
  "message": "Bienvenido a rsc.finance\n\nPor favor, firma este mensaje..."
}
```

**Rate Limit**: 10 requests/minuto por wallet

---

### 2. Verificar Firma y Autenticarse

**POST** `/api/auth/verify`

**Body**:
```json
{
  "walletAddress": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
  "nonce": "0x...",
  "signature": "0x..."
}
```

**Response**:
```json
{
  "accessToken": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refreshToken": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user": {
    "id": "uuid",
    "walletAddress": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
    "createdAt": "2025-12-25T21:50:54.296Z"
  }
}
```

**Rate Limit**: 5 requests/minuto por wallet

---

### 3. Refrescar Token

**POST** `/api/auth/refresh`

**Headers**: `Authorization: Bearer {refreshToken}`

**Body**:
```json
{
  "refreshToken": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

**Response**:
```json
{
  "accessToken": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

---

### 4. Obtener Perfil del Usuario Autenticado

**GET** `/api/auth/me`

**Headers**: `Authorization: Bearer {accessToken}`

**Response**:
```json
{
  "id": "uuid",
  "walletAddress": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
  "isActive": true,
  "loginCount": 5,
  "lastLoginAt": "2025-12-25T21:50:54.296Z",
  "createdAt": "2025-12-25T21:50:54.296Z"
}
```

---

### 5. Cerrar Sesión

**POST** `/api/auth/logout`

**Headers**: `Authorization: Bearer {accessToken}`

**Response**:
```json
{
  "message": "Sesión cerrada exitosamente"
}
```

---

## 👥 Usuarios

### 1. Listar Usuarios (Público)

**GET** `/api/users?page=1&limit=20&search=0x...`

**Query Parameters**:
- `page` (default: 1)
- `limit` (default: 20)
- `search` (opcional, busca por wallet address)

**Response**:
```json
{
  "data": [
    {
      "id": "uuid",
      "wallet_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
      "reputation_score": 100.50,
      "created_at": "2025-12-25T21:50:54.296Z"
    }
  ],
  "total": 50,
  "page": 1,
  "limit": 20,
  "totalPages": 3
}
```

---

### 2. Obtener Usuario por ID (Público)

**GET** `/api/users/:id`

**Response**:
```json
{
  "id": "uuid",
  "wallet_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
  "reputation_score": 100.50,
  "created_at": "2025-12-25T21:50:54.296Z"
}
```

---

### 3. Obtener Usuario por Wallet Address (Público)

**GET** `/api/users/wallet/:address`

**Ejemplo**: `GET /api/users/wallet/0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb`

**Response**: Igual que obtener por ID

---

### 4. Ranking de Usuarios (Público)

**GET** `/api/users/ranking?limit=100`

**Response**:
```json
[
  {
    "id": "uuid",
    "wallet_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
    "reputation_score": 150.75,
    "created_at": "2025-12-25T21:50:54.296Z"
  }
]
```

---

### 5. Estadísticas de Usuario (Público)

**GET** `/api/users/stats/:address`

**Response**:
```json
{
  "walletAddress": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
  "totalOrders": 25,
  "completedOrders": 20,
  "cancelledOrders": 3,
  "disputedOrders": 2,
  "averageRating": 4.5
}
```

---

### 6. Perfil Completo del Usuario Autenticado

**GET** `/api/users/me/profile`

**Headers**: `Authorization: Bearer {accessToken}`

**Response**:
```json
{
  "id": "uuid",
  "walletAddress": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
  "reputationScore": 100.50,
  "createdAt": "2025-12-25T21:50:54.296Z",
  "isActive": true,
  "lastLoginAt": "2025-12-25T21:50:54.296Z",
  "loginCount": 5
}
```

---

## 💼 Órdenes P2P

### 1. Crear Orden (Vender)

**POST** `/api/orders`

**Headers**: `Authorization: Bearer {accessToken}`

**Body**:
```json
{
  "cryptoCurrency": "ETH",
  "cryptoAmount": "0.5",
  "fiatCurrency": "USD",
  "fiatAmount": "1500.00",
  "pricePerUnit": "3000.00",
  "paymentMethod": "BANK_TRANSFER",
  "terms": "Pago por transferencia bancaria. Entrega inmediata.",
  "expiresAt": "2025-12-30T23:59:59.000Z"
}
```

**Response**:
```json
{
  "id": "uuid",
  "sellerId": "uuid",
  "cryptoCurrency": "ETH",
  "cryptoAmount": "0.5",
  "fiatCurrency": "USD",
  "fiatAmount": "1500.00",
  "pricePerUnit": "3000.00",
  "status": "CREATED",
  "paymentMethod": "BANK_TRANSFER",
  "terms": "Pago por transferencia bancaria...",
  "expiresAt": "2025-12-30T23:59:59.000Z",
  "createdAt": "2025-12-25T21:50:54.296Z"
}
```

**Rate Limit**: 10 requests/minuto

---

### 2. Listar Órdenes (Público)

**GET** `/api/orders?page=1&limit=20&status=CREATED&cryptoCurrency=ETH&fiatCurrency=USD`

**Query Parameters**:
- `page` (default: 1)
- `limit` (default: 20)
- `status` (opcional): `CREATED`, `AWAITING_FUNDS`, `ONCHAIN_LOCKED`, `COMPLETED`, `REFUNDED`, `DISPUTED`
- `sellerId` (opcional)
- `buyerId` (opcional)
- `cryptoCurrency` (opcional)
- `fiatCurrency` (opcional)

**Response**:
```json
{
  "data": [
    {
      "id": "uuid",
      "seller": {
        "id": "uuid",
        "wallet_address": "0x...",
        "reputation_score": 100.50
      },
      "buyer": null,
      "cryptoCurrency": "ETH",
      "cryptoAmount": "0.5",
      "fiatCurrency": "USD",
      "fiatAmount": "1500.00",
      "pricePerUnit": "3000.00",
      "status": "CREATED",
      "paymentMethod": "BANK_TRANSFER",
      "terms": "...",
      "expiresAt": "2025-12-30T23:59:59.000Z",
      "createdAt": "2025-12-25T21:50:54.296Z"
    }
  ],
  "total": 100,
  "page": 1,
  "limit": 20,
  "totalPages": 5
}
```

**Rate Limit**: 30 requests/minuto

---

### 3. Obtener Orden por ID (Público)

**GET** `/api/orders/:id`

**Response**: Objeto de orden completo

---

### 4. Obtener Estado de Orden (Público)

**GET** `/api/orders/:id/status`

**Response**:
```json
{
  "id": "uuid",
  "status": "CREATED",
  "escrowId": null,
  "updatedAt": "2025-12-25T21:50:54.296Z"
}
```

---

### 5. Aceptar Orden (Comprador)

**PUT** `/api/orders/:id/accept`

**Headers**: `Authorization: Bearer {accessToken}`

**Body** (opcional):
```json
{
  "paymentMethod": "BANK_TRANSFER"
}
```

**Response**: Orden actualizada con `buyerId` y `status: AWAITING_FUNDS`

**Rate Limit**: 5 requests/minuto

---

### 6. Cancelar Orden

**PUT** `/api/orders/:id/cancel`

**Headers**: `Authorization: Bearer {accessToken}`

**Response**: Orden cancelada

**Rate Limit**: 10 requests/minuto

---

### 7. Mis Órdenes

**GET** `/api/orders/me?role=seller&status=CREATED&page=1&limit=20`

**Headers**: `Authorization: Bearer {accessToken}`

**Query Parameters**:
- `role`: `seller`, `buyer`, o `both` (default: both)
- `status` (opcional)
- `page` (default: 1)
- `limit` (default: 20)

**Response**: Lista de órdenes del usuario

**Rate Limit**: 20 requests/minuto

---

### 8. Completar Orden

**PUT** `/api/orders/:id/complete`

**Headers**: `Authorization: Bearer {accessToken}`

**Response**: Orden marcada como completada

**Rate Limit**: 5 requests/minuto

---

### 9. Marcar Orden como Disputada

**PUT** `/api/orders/:id/dispute`

**Headers**: `Authorization: Bearer {accessToken}`

**Response**: Orden marcada como disputada

**Nota**: La disputa real se crea en `/api/disputes`

**Rate Limit**: 3 requests/minuto

---

### 10. Marcar Orden como Bloqueada (Fondos en Escrow)

**PUT** `/api/orders/:id/mark-locked`

**Headers**: `Authorization: Bearer {accessToken}`

**Response**: Orden marcada como `ONCHAIN_LOCKED`

**Rate Limit**: 5 requests/minuto

---

## 🔒 Escrow (Mapeo Order ↔ Blockchain)

### 1. Crear Mapeo Order ↔ Escrow

**POST** `/api/escrow`

**Body**:
```json
{
  "orderId": "uuid",
  "escrowId": "0x...",
  "contractAddress": "0x...",
  "cryptoAmount": "0.5",
  "cryptoCurrency": "ETH",
  "createTransactionHash": "0x..."
}
```

**Response**: Escrow creado

---

### 2. Obtener Escrow por ID

**GET** `/api/escrow/:id`

**Response**:
```json
{
  "id": "uuid",
  "orderId": "uuid",
  "escrowId": "0x...",
  "contractAddress": "0x...",
  "cryptoAmount": "0.5",
  "cryptoCurrency": "ETH",
  "status": "LOCKED",
  "lockedAt": "2025-12-25T21:50:54.296Z",
  "createdAt": "2025-12-25T21:50:54.296Z"
}
```

---

### 3. Obtener Escrow por Order ID

**GET** `/api/escrow/order/:orderId`

**Response**: Escrow asociado a la orden

---

### 4. Obtener Escrow por Escrow ID (Blockchain)

**GET** `/api/escrow/blockchain/:escrowId`

**Response**: Escrow encontrado

---

### 5. Obtener Mapeo

**GET** `/api/escrow/mapping?orderId=xxx` o `?escrowId=xxx`

**Response**: Mapeo order_id ↔ escrow_id

---

### 6. Validar Consistencia

**GET** `/api/escrow/validate/:orderId`

**Response**: Validación de consistencia entre orden y escrow

---

### 7. Listar Escrows

**GET** `/api/escrow?orderId=xxx&escrowId=xxx&status=LOCKED`

**Query Parameters**:
- `orderId` (opcional)
- `escrowId` (opcional)
- `status` (opcional): `PENDING`, `LOCKED`, `RELEASED`, `REFUNDED`, `DISPUTED`

---

### 8. Actualizar Escrow

**PUT** `/api/escrow/:escrowId`

**Body**:
```json
{
  "status": "RELEASED",
  "releaseTransactionHash": "0x...",
  "releasedAt": "2025-12-25T21:50:54.296Z"
}
```

---

## ⚖️ Disputas

### 1. Crear Disputa

**POST** `/api/disputes`

**Headers**: `Authorization: Bearer {accessToken}`

**Body**:
```json
{
  "orderId": "uuid",
  "reason": "El vendedor no entregó el producto",
  "responseDeadline": "2025-12-27T23:59:59.000Z",
  "evidenceDeadline": "2025-12-28T23:59:59.000Z"
}
```

**Response**: Disputa creada

---

### 2. Listar Disputas

**GET** `/api/disputes?status=OPEN&orderId=xxx&userId=xxx`

**Query Parameters**:
- `status` (opcional): `OPEN`, `IN_REVIEW`, `RESOLVED`, `CLOSED`, `ESCALATED`
- `orderId` (opcional)
- `userId` (opcional)

---

### 3. Obtener Disputa por ID

**GET** `/api/disputes/:id`

**Response**: Disputa completa con evidencia

---

### 4. Agregar Evidencia

**POST** `/api/disputes/:id/evidence`

**Headers**: `Authorization: Bearer {accessToken}`

**Body**:
```json
{
  "evidenceType": "IMAGE",
  "evidenceUrl": "https://...",
  "description": "Captura de pantalla del problema",
  "metadata": "{}"
}
```

---

### 5. Resolver Disputa

**PUT** `/api/disputes/:id/resolve`

**Headers**: `Authorization: Bearer {accessToken}`

**Body**:
```json
{
  "resolution": "Favor del comprador",
  "resolvedBy": "admin"
}
```

---

### 6. Cerrar Disputa

**PUT** `/api/disputes/:id/close`

**Body**:
```json
{
  "escrowResolution": "RELEASED"
}
```

---

### 7. Escalar Disputa

**PUT** `/api/disputes/:id/escalate`

**Headers**: `Authorization: Bearer {accessToken}`

---

### 8. Disputas Próximas a Expirar

**GET** `/api/disputes/expiring?hours=24`

---

## ⭐ Reputación

### 1. Obtener Reputación de Usuario

**GET** `/api/reputation/:userId`

**Response**:
```json
{
  "userId": "uuid",
  "reputationScore": 100.50,
  "totalEvents": 25,
  "positiveEvents": 20,
  "negativeEvents": 5
}
```

---

### 2. Historial de Reputación

**GET** `/api/reputation/:userId/history?limit=50`

**Response**: Lista de eventos de reputación

---

### 3. Estadísticas de Reputación

**GET** `/api/reputation/:userId/stats`

**Response**: Estadísticas detalladas

---

### 4. Recalcular Reputación

**POST** `/api/reputation/:userId/recalculate`

---

### 5. Ranking de Reputación

**GET** `/api/reputation/ranking?limit=100`

---

### 6. Aplicar Penalización

**POST** `/api/reputation/penalty`

**Body**:
```json
{
  "userId": "uuid",
  "reason": "Orden cancelada sin razón",
  "orderId": "uuid",
  "disputeId": "uuid"
}
```

---

### 7. Aplicar Bonus

**POST** `/api/reputation/bonus`

**Body**: Similar a penalty

---

## 🔔 Notificaciones

### 1. Obtener Notificaciones

**GET** `/api/notifications?limit=50&unreadOnly=false`

**Headers**: `Authorization: Bearer {accessToken}`

**Query Parameters**:
- `limit` (default: 50)
- `unreadOnly` (default: false)

**Response**:
```json
[
  {
    "id": "uuid",
    "type": "ORDER_ACCEPTED",
    "title": "Orden aceptada",
    "message": "Tu orden ha sido aceptada por 0x...",
    "read": false,
    "orderId": "uuid",
    "createdAt": "2025-12-25T21:50:54.296Z"
  }
]
```

---

### 2. Contador de No Leídas

**GET** `/api/notifications/unread-count`

**Headers**: `Authorization: Bearer {accessToken}`

**Response**:
```json
{
  "count": 5
}
```

---

### 3. Marcar como Leída

**PUT** `/api/notifications/:id/read`

**Headers**: `Authorization: Bearer {accessToken}`

---

### 4. Marcar Todas como Leídas

**PUT** `/api/notifications/read-all`

**Headers**: `Authorization: Bearer {accessToken}`

---

## 🔗 Blockchain (Opcional)

### 1. Estado de Blockchain

**GET** `/api/blockchain/status`

**Response**: Estado de sincronización

---

### 2. Iniciar Sincronización

**POST** `/api/blockchain/sync/start`

---

### 3. Detener Sincronización

**POST** `/api/blockchain/sync/stop`

---

### 4. Re-sincronizar desde Bloque

**POST** `/api/blockchain/sync/resync/:blockNumber`

---

### 5. Auto Re-sincronizar

**POST** `/api/blockchain/sync/auto-resync`

---

### 6. Reconciliar Todos

**POST** `/api/blockchain/reconcile/all`

---

### 7. Reconciliar Escrow

**POST** `/api/blockchain/reconcile/escrow/:escrowId`

---

### 8. Validar Bloque

**GET** `/api/blockchain/validate/block/:blockNumber`

---

### 9. Último Bloque

**GET** `/api/blockchain/latest-block`

---

### 10. Balance de Wallet

**GET** `/api/blockchain/balance/:address`

---

## ❤️ Health Checks

### 1. Health General

**GET** `/api/health`

**Response**:
```json
{
  "status": "healthy",
  "timestamp": "2025-12-25T21:50:54.296Z",
  "uptime": 400.13,
  "checks": {
    "database": {
      "status": "ok",
      "latency": 30
    },
    "redis": {
      "status": "ok",
      "latency": 1
    },
    "blockchain": {
      "status": "ok",
      "details": {
        "message": "Blockchain not configured"
      }
    }
  }
}
```

---

### 2. Liveness

**GET** `/api/health/live`

---

### 3. Readiness

**GET** `/api/health/ready`

---

## 📡 WebSockets

### Conexión

**URL**: `ws://64.23.151.47:3000` (o tu dominio)

**Autenticación**: Enviar token JWT en query string o headers

### Eventos Disponibles

#### Market Gateway (`/market`)

- **subscribe**: Suscribirse a actualizaciones de mercado
- **unsubscribe**: Desuscribirse
- **subscribe:user**: Suscribirse a actualizaciones de usuario específico

**Eventos emitidos**:
- `order:created`
- `order:updated`
- `order:accepted`
- `order:cancelled`
- `order:completed`

#### Notifications Gateway (`/notifications`)

- **subscribe**: Suscribirse a notificaciones
- **unsubscribe**: Desuscribirse
- **subscribe:user**: Suscribirse a notificaciones de usuario
- **notification:read**: Marcar notificación como leída

**Eventos emitidos**:
- `notification`: Nueva notificación

---

## 📊 Estructura de Datos

### User (Usuario)

```typescript
{
  id: string;                    // UUID
  walletAddress: string;         // Dirección Ethereum (única)
  reputationScore: number;        // Puntuación de reputación
  isActive: boolean;             // Estado activo
  lastLoginAt: Date | null;      // Último login
  loginCount: number;            // Contador de logins
  createdAt: Date;                // Fecha de creación
  updatedAt: Date;                // Fecha de actualización
}
```

---

### Order (Orden P2P)

```typescript
{
  id: string;                    // UUID
  sellerId: string;              // ID del vendedor
  buyerId: string | null;        // ID del comprador (null hasta aceptar)
  cryptoCurrency: string;        // Ej: "ETH", "BTC", "USDT"
  cryptoAmount: string;          // Cantidad de crypto (decimal)
  fiatCurrency: string;          // Ej: "USD", "EUR"
  fiatAmount: string;            // Cantidad de fiat (decimal)
  pricePerUnit: string | null;  // Precio por unidad
  status: OrderStatus;          // CREATED, AWAITING_FUNDS, ONCHAIN_LOCKED, COMPLETED, REFUNDED, DISPUTED
  escrowId: string | null;       // ID del escrow en blockchain
  paymentMethod: string | null; // Método de pago
  terms: string | null;          // Términos y condiciones
  expiresAt: Date | null;        // Fecha de expiración
  acceptedAt: Date | null;       // Fecha de aceptación
  completedAt: Date | null;      // Fecha de completación
  cancelledAt: Date | null;      // Fecha de cancelación
  cancelledBy: string | null;   // SELLER o BUYER
  disputedAt: Date | null;       // Fecha de disputa
  createdAt: Date;               // Fecha de creación
  updatedAt: Date;               // Fecha de actualización
}
```

**Estados de Orden**:
- `CREATED`: Orden creada, esperando comprador
- `AWAITING_FUNDS`: Comprador aceptó, esperando fondos en escrow
- `ONCHAIN_LOCKED`: Fondos bloqueados en escrow
- `COMPLETED`: Orden completada exitosamente
- `REFUNDED`: Fondos devueltos
- `DISPUTED`: Orden en disputa

---

### Escrow

```typescript
{
  id: string;                    // UUID
  orderId: string;               // ID de la orden (único)
  escrowId: string;              // ID del escrow en blockchain (único)
  contractAddress: string;       // Dirección del contrato
  cryptoAmount: string;           // Cantidad de crypto
  cryptoCurrency: string;         // Moneda crypto
  status: EscrowStatus;          // PENDING, LOCKED, RELEASED, REFUNDED, DISPUTED
  createTransactionHash: string | null;
  releaseTransactionHash: string | null;
  refundTransactionHash: string | null;
  lockedAt: Date | null;
  releasedAt: Date | null;
  refundedAt: Date | null;
  createdAt: Date;
  updatedAt: Date;
}
```

---

### Dispute (Disputa)

```typescript
{
  id: string;                    // UUID
  orderId: string;               // ID de la orden
  initiatorId: string;           // ID del usuario que inició la disputa
  respondentId: string | null;   // ID del otro usuario
  reason: string;                // Razón de la disputa
  status: DisputeStatus;         // OPEN, IN_REVIEW, RESOLVED, CLOSED, ESCALATED
  resolution: string | null;     // Resolución
  resolvedBy: string | null;     // Quién resolvió
  resolvedAt: Date | null;        // Fecha de resolución
  escalatedAt: Date | null;       // Fecha de escalación
  expiresAt: Date | null;        // Fecha de expiración
  responseDeadline: Date | null; // Deadline para respuesta
  evidenceDeadline: Date | null; // Deadline para evidencia
  escrowResolution: string | null;
  escrowResolvedAt: Date | null;
  createdAt: Date;
  updatedAt: Date;
}
```

---

### Notification (Notificación)

```typescript
{
  id: string;                    // UUID
  userId: string;                // ID del usuario
  type: NotificationType;        // Tipo de notificación
  title: string;                 // Título
  message: string;               // Mensaje
  read: boolean;                 // Leída o no
  readAt: Date | null;           // Fecha de lectura
  data: object | null;           // Datos adicionales (JSON)
  orderId: string | null;        // ID de orden relacionada
  disputeId: string | null;      // ID de disputa relacionada
  escrowId: string | null;       // ID de escrow relacionada
  createdAt: Date;                // Fecha de creación
}
```

**Tipos de Notificación**:
- `ORDER_CREATED`
- `ORDER_ACCEPTED`
- `ORDER_COMPLETED`
- `ORDER_CANCELLED`
- `ORDER_DISPUTED`
- `DISPUTE_OPENED`
- `DISPUTE_RESOLVED`
- `ESCROW_LOCKED`
- `ESCROW_RELEASED`
- `ESCROW_REFUNDED`
- `MARKET_UPDATE`
- `PRICE_UPDATE`
- `REPUTATION_CHANGE`

---

### ReputationEvent (Evento de Reputación)

```typescript
{
  id: string;                    // UUID
  userId: string;                // ID del usuario
  eventType: ReputationEventType; // Tipo de evento
  scoreChange: number;           // Cambio en la puntuación
  previousScore: number;         // Puntuación anterior
  newScore: number;              // Nueva puntuación
  orderId: string | null;        // ID de orden relacionada
  disputeId: string | null;      // ID de disputa relacionada
  reason: string | null;         // Razón del cambio
  metadata: string | null;       // Metadatos adicionales
  createdAt: Date;               // Fecha del evento
}
```

**Tipos de Eventos**:
- `TRADE_COMPLETED`: Transacción completada (+)
- `TRADE_CANCELLED`: Transacción cancelada (-)
- `DISPUTE_OPENED`: Disputa abierta (-)
- `DISPUTE_RESOLVED_FAVOR`: Disputa resuelta a favor (+)
- `DISPUTE_RESOLVED_AGAINST`: Disputa resuelta en contra (-)
- `PENALTY`: Penalización manual (-)
- `BONUS`: Bonus manual (+)

---

## 🔒 Headers Requeridos

### Autenticación

Para endpoints protegidos, incluir:

```
Authorization: Bearer {accessToken}
```

### Content-Type

```
Content-Type: application/json
```

---

## ⚡ Rate Limiting

Todos los endpoints tienen rate limiting configurado:

- **Auth Challenge**: 10/min por wallet
- **Auth Verify**: 5/min por wallet
- **Crear Orden**: 10/min
- **Aceptar Orden**: 5/min
- **Listar Órdenes**: 30/min
- **Mis Órdenes**: 20/min
- **General**: 100/min por IP

---

## 🚨 Códigos de Error

- `200`: OK
- `201`: Created
- `400`: Bad Request (validación fallida)
- `401`: Unauthorized (token inválido o expirado)
- `403`: Forbidden (sin permisos)
- `404`: Not Found
- `429`: Too Many Requests (rate limit excedido)
- `500`: Internal Server Error

---

## 📝 Ejemplos de Uso

### Flujo Completo: Crear y Aceptar Orden

```javascript
// 1. Autenticarse
const challengeResponse = await fetch('http://64.23.151.47:3000/api/auth/challenge', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ walletAddress: '0x...' })
});
const { nonce, message } = await challengeResponse.json();

// 2. Firmar mensaje con wallet (MetaMask, WalletConnect, etc.)
const signature = await wallet.signMessage(message);

// 3. Verificar firma
const verifyResponse = await fetch('http://64.23.151.47:3000/api/auth/verify', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ walletAddress: '0x...', nonce, signature })
});
const { accessToken, refreshToken, user } = await verifyResponse.json();

// 4. Crear orden
const orderResponse = await fetch('http://64.23.151.47:3000/api/orders', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${accessToken}`
  },
  body: JSON.stringify({
    cryptoCurrency: 'ETH',
    cryptoAmount: '0.5',
    fiatCurrency: 'USD',
    fiatAmount: '1500.00',
    pricePerUnit: '3000.00',
    paymentMethod: 'BANK_TRANSFER',
    terms: 'Pago por transferencia bancaria',
    expiresAt: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString()
  })
});
const order = await orderResponse.json();

// 5. Otro usuario acepta la orden
const acceptResponse = await fetch(`http://64.23.151.47:3000/api/orders/${order.data.id}/accept`, {
  method: 'PUT',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${buyerAccessToken}`
  }
});
```

---

## 🔗 URLs Importantes

- **Base URL**: `http://64.23.151.47:3000/api`
- **Health Check**: `http://64.23.151.47:3000/api/health`
- **WebSocket**: `ws://64.23.151.47:3000`

---

## 📞 Soporte

Para más información, consulta:
- `ARQUITECTURA-P2P.md`: Arquitectura completa del sistema
- `GUIA-CONFIGURACION-DB.md`: Configuración de base de datos

