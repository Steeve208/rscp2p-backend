# Módulo de Orders (Núcleo del Mercado P2P)

Sistema completo de gestión de órdenes P2P con estados off-chain.

## Características

- ✅ Crear ofertas P2P
- ✅ Aceptar ofertas
- ✅ Cancelar órdenes
- ✅ Gestión de estados off-chain
- ✅ Integración con sistema de reputation
- ✅ Validaciones de permisos y estados

## Estados de Orden

El sistema maneja los siguientes estados:

1. **CREATED**: Orden creada, esperando comprador
2. **AWAITING_FUNDS**: Comprador aceptó, esperando fondos on-chain
3. **ONCHAIN_LOCKED**: Fondos bloqueados en escrow on-chain
4. **COMPLETED**: Orden completada exitosamente
5. **REFUNDED**: Orden cancelada/reembolsada
6. **DISPUTED**: Orden en disputa

### Transiciones de Estado

```
CREATED → AWAITING_FUNDS (cuando comprador acepta)
AWAITING_FUNDS → ONCHAIN_LOCKED (cuando se bloquean fondos)
AWAITING_FUNDS → REFUNDED (si se cancela)
ONCHAIN_LOCKED → COMPLETED (cuando se completa)
ONCHAIN_LOCKED → DISPUTED (si hay disputa)
AWAITING_FUNDS → DISPUTED (si hay disputa)
```

## Estructura de Datos

### Entidad Order

```typescript
{
  id: string;                    // UUID
  sellerId: string;             // ID del vendedor
  buyerId: string;               // ID del comprador (null hasta aceptar)
  cryptoAmount: number;           // Cantidad de crypto
  cryptoCurrency: string;         // Moneda crypto (BTC, ETH, etc.)
  fiatAmount: number;            // Cantidad en fiat
  fiatCurrency: string;          // Moneda fiat (USD, EUR, etc.)
  pricePerUnit: number;          // Precio por unidad
  status: OrderStatus;            // Estado actual
  escrowId: string;              // ID del escrow on-chain
  paymentMethod: string;          // Método de pago
  terms: string;                 // Términos de la orden
  expiresAt: Date;               // Fecha de expiración
  acceptedAt: Date;              // Fecha de aceptación
  completedAt: Date;             // Fecha de completación
  cancelledAt: Date;              // Fecha de cancelación
  cancelledBy: string;            // Quién canceló (SELLER/BUYER)
  createdAt: Date;               // Fecha de creación
  updatedAt: Date;               // Última actualización
}
```

## Endpoints

### Crear Oferta

```http
POST /api/orders
Authorization: Bearer <token>
Content-Type: application/json

{
  "cryptoAmount": 0.5,
  "cryptoCurrency": "BTC",
  "fiatAmount": 20000,
  "fiatCurrency": "USD",
  "pricePerUnit": 40000,
  "paymentMethod": "Bank Transfer",
  "terms": "Payment within 24 hours",
  "expiresAt": "2024-12-31T23:59:59Z"
}
```

**Respuesta:**
```json
{
  "id": "uuid",
  "seller_id": "uuid",
  "buyer_id": null,
  "crypto_amount": 0.5,
  "crypto_currency": "BTC",
  "fiat_amount": 20000,
  "fiat_currency": "USD",
  "price_per_unit": 40000,
  "status": "CREATED",
  "created_at": "2024-01-01T00:00:00.000Z"
}
```

### Listar Órdenes (Público)

```http
GET /api/orders?page=1&limit=20&status=CREATED&cryptoCurrency=BTC&fiatCurrency=USD
```

**Parámetros de consulta:**
- `page`: Número de página (default: 1)
- `limit`: Resultados por página (default: 20)
- `status`: Filtrar por estado
- `sellerId`: Filtrar por vendedor
- `buyerId`: Filtrar por comprador
- `cryptoCurrency`: Filtrar por moneda crypto
- `fiatCurrency`: Filtrar por moneda fiat

**Respuesta:**
```json
{
  "data": [...],
  "total": 100,
  "page": 1,
  "limit": 20,
  "totalPages": 5
}
```

### Obtener Orden por ID (Público)

```http
GET /api/orders/:id
```

### Aceptar Oferta

```http
PUT /api/orders/:id/accept
Authorization: Bearer <token>
Content-Type: application/json

{
  "paymentMethod": "PayPal"
}
```

**Respuesta:**
```json
{
  "id": "uuid",
  "status": "AWAITING_FUNDS",
  "buyer_id": "uuid",
  "accepted_at": "2024-01-01T10:00:00.000Z"
}
```

### Cancelar Orden

```http
PUT /api/orders/:id/cancel
Authorization: Bearer <token>
```

**Reglas:**
- Solo el vendedor o comprador pueden cancelar
- Solo se puede cancelar en estados `CREATED` o `AWAITING_FUNDS`
- Penalización de reputation: -10 puntos

**Respuesta:**
```json
{
  "id": "uuid",
  "status": "REFUNDED",
  "cancelled_at": "2024-01-01T11:00:00.000Z",
  "cancelled_by": "SELLER"
}
```

### Mis Órdenes

```http
GET /api/orders/me?role=seller&status=CREATED
Authorization: Bearer <token>
```

**Parámetros:**
- `role`: `seller`, `buyer`, o `both` (default: `both`)
- `status`: Filtrar por estado

### Completar Orden

```http
PUT /api/orders/:id/complete
Authorization: Bearer <token>
```

**Reglas:**
- Solo se puede completar en estado `ONCHAIN_LOCKED`
- Solo el vendedor o comprador pueden completar
- Bonificación de reputation: +5 puntos para ambos

### Disputar Orden

```http
PUT /api/orders/:id/dispute
Authorization: Bearer <token>
```

**Reglas:**
- Solo se puede disputar en estados `AWAITING_FUNDS` o `ONCHAIN_LOCKED`

## Flujo Completo de una Orden

### 1. Vendedor Crea Oferta
```typescript
POST /api/orders
{
  cryptoAmount: 0.5,
  cryptoCurrency: "BTC",
  fiatAmount: 20000,
  fiatCurrency: "USD"
}
// Estado: CREATED
```

### 2. Comprador Acepta
```typescript
PUT /api/orders/:id/accept
// Estado: AWAITING_FUNDS
```

### 3. Fondos Bloqueados On-Chain
```typescript
// Llamado por el módulo de escrow cuando se bloquean fondos
ordersService.markAsOnChainLocked(orderId)
// Estado: ONCHAIN_LOCKED
```

### 4. Orden Completada
```typescript
PUT /api/orders/:id/complete
// Estado: COMPLETED
// Reputation: +5 para vendedor y comprador
```

## Integración con Reputation

El sistema actualiza automáticamente el reputation score:

- ✅ **Completar orden**: +5 puntos (vendedor y comprador)
- ❌ **Cancelar orden**: -10 puntos (quien cancela)
- ✅ **Resolver disputa a favor**: +10 puntos (módulo de disputes)
- ❌ **Disputa infundada**: -15 puntos (módulo de disputes)

## Validaciones

### Crear Orden
- Vendedor debe existir y estar activo
- Cantidades deben ser positivas
- Monedas deben ser válidas
- `pricePerUnit` se calcula automáticamente si no se proporciona

### Aceptar Orden
- Orden debe estar en estado `CREATED`
- Comprador no puede ser el mismo que vendedor
- Orden no debe estar expirada
- Comprador debe existir y estar activo

### Cancelar Orden
- Usuario debe ser vendedor o comprador
- Solo estados `CREATED` o `AWAITING_FUNDS` permiten cancelación

### Completar Orden
- Orden debe estar en estado `ONCHAIN_LOCKED`
- Usuario debe ser vendedor o comprador

## Ejemplo de Uso

```typescript
// 1. Vendedor crea oferta
const order = await fetch('/api/orders', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${sellerToken}`,
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    cryptoAmount: 0.5,
    cryptoCurrency: 'BTC',
    fiatAmount: 20000,
    fiatCurrency: 'USD',
    paymentMethod: 'Bank Transfer',
  }),
}).then(r => r.json());

// 2. Comprador acepta
const accepted = await fetch(`/api/orders/${order.id}/accept`, {
  method: 'PUT',
  headers: {
    'Authorization': `Bearer ${buyerToken}`,
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    paymentMethod: 'PayPal',
  }),
}).then(r => r.json());

// 3. Ver mis órdenes como vendedor
const myOrders = await fetch('/api/orders/me?role=seller', {
  headers: {
    'Authorization': `Bearer ${sellerToken}`,
  },
}).then(r => r.json());
```

## Notas Importantes

1. **Estados Off-Chain**: Todos los estados se manejan off-chain. El estado `ONCHAIN_LOCKED` se actualiza cuando el módulo de escrow bloquea fondos.

2. **Expiración**: Las órdenes pueden tener fecha de expiración. Una vez expirada, no se pueden aceptar.

3. **Reputation**: El sistema actualiza automáticamente el reputation score al completar o cancelar órdenes.

4. **Permisos**: Solo el vendedor o comprador pueden realizar acciones sobre sus órdenes.

5. **Validación de Estados**: Cada transición de estado está validada para asegurar flujos correctos.
