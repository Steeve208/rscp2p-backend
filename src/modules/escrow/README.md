# Módulo de Escrow

Puente lógico entre órdenes y blockchain. Mapea `order_id ↔ escrow_id` y valida consistencia.

## Características

- ✅ Mapeo bidireccional `order_id ↔ escrow_id`
- ✅ Validación de consistencia entre orden y escrow
- ✅ Sincronización de estados
- ✅ **NO ejecuta transacciones blockchain** (solo mapeo y validación)
- ✅ Validación de datos (cantidad, moneda, estados)

## Principio Fundamental

**Este módulo NO ejecuta transacciones blockchain.** Solo actúa como:
- Registro del mapeo entre órdenes off-chain y escrows on-chain
- Validador de consistencia entre ambos sistemas
- Sincronizador de estados

Las transacciones blockchain se ejecutan en otro módulo (blockchain service).

## Estructura de Datos

### Entidad Escrow

```typescript
{
  id: string;                    // UUID interno
  orderId: string;               // ID de la orden (único)
  escrowId: string;               // ID del escrow en blockchain (único)
  contractAddress: string;        // Dirección del contrato escrow
  cryptoAmount: number;           // Cantidad de crypto
  cryptoCurrency: string;         // Moneda crypto
  status: EscrowStatus;          // Estado del escrow
  createTransactionHash: string;  // Hash de creación
  releaseTransactionHash: string; // Hash de liberación
  refundTransactionHash: string; // Hash de reembolso
  lockedAt: Date;                // Fecha de bloqueo
  releasedAt: Date;              // Fecha de liberación
  refundedAt: Date;              // Fecha de reembolso
  validationErrors: string;      // Errores de validación
  createdAt: Date;              // Fecha de creación
  updatedAt: Date;               // Última actualización
}
```

### Estados de Escrow

- **PENDING**: Escrow creado, esperando fondos
- **LOCKED**: Fondos bloqueados en escrow
- **RELEASED**: Fondos liberados al vendedor
- **REFUNDED**: Fondos reembolsados al comprador
- **DISPUTED**: Escrow en disputa

## Endpoints

### Crear Mapeo

```http
POST /api/escrow
Content-Type: application/json

{
  "orderId": "uuid",
  "escrowId": "0x...",
  "contractAddress": "0x...",
  "cryptoAmount": 0.5,
  "cryptoCurrency": "BTC",
  "createTransactionHash": "0x..."
}
```

**Validaciones:**
- La orden debe existir
- No debe existir ya un escrow para esta orden
- El `escrowId` no debe estar en uso
- Valida consistencia de cantidad y moneda con la orden

**Respuesta:**
```json
{
  "id": "uuid",
  "order_id": "uuid",
  "escrow_id": "0x...",
  "contract_address": "0x...",
  "crypto_amount": 0.5,
  "crypto_currency": "BTC",
  "status": "PENDING",
  "created_at": "2024-01-01T00:00:00.000Z"
}
```

### Obtener Escrow por ID

```http
GET /api/escrow/:id
```

### Obtener Escrow por Order ID

```http
GET /api/escrow/order/:orderId
```

### Obtener Escrow por Escrow ID (Blockchain)

```http
GET /api/escrow/blockchain/:escrowId
```

### Obtener Mapeo

```http
GET /api/escrow/mapping?orderId=xxx
GET /api/escrow/mapping?escrowId=0x...
```

**Respuesta:**
```json
{
  "orderId": "uuid",
  "escrowId": "0x..."
}
```

### Validar Consistencia

```http
GET /api/escrow/validate/:orderId
```

**Respuesta:**
```json
{
  "isValid": true,
  "errors": [],
  "warnings": [],
  "orderId": "uuid",
  "escrowId": "0x..."
}
```

**Validaciones realizadas:**
- Cantidad de crypto coincide
- Moneda coincide
- Estados son consistentes
- Existencia de registros

### Listar Escrows

```http
GET /api/escrow?orderId=xxx&escrowId=xxx&status=LOCKED
```

### Actualizar Estado

```http
PUT /api/escrow/:escrowId
Content-Type: application/json

{
  "status": "LOCKED",
  "releaseTransactionHash": "0x...",
  "refundTransactionHash": "0x..."
}
```

**Nota:** Este endpoint actualiza el estado cuando se ejecutan transacciones en blockchain. **NO ejecuta las transacciones**, solo registra el resultado.

## Flujo de Integración

### 1. Crear Escrow en Blockchain
```typescript
// En el módulo de blockchain (fuera de este módulo)
const escrowId = await blockchainService.createEscrow(orderData);
const txHash = await blockchainService.getTransactionHash();
```

### 2. Registrar Mapeo
```typescript
// Registrar el mapeo en este módulo
await escrowService.create({
  orderId: order.id,
  escrowId: escrowId,
  contractAddress: contractAddress,
  cryptoAmount: order.cryptoAmount,
  cryptoCurrency: order.cryptoCurrency,
  createTransactionHash: txHash,
});
```

### 3. Validar Consistencia
```typescript
// Validar que todo coincide
const validation = await escrowService.validateConsistency(order.id);
if (!validation.isValid) {
  // Manejar errores
}
```

### 4. Actualizar Estado cuando se Bloquean Fondos
```typescript
// Cuando se bloquean fondos en blockchain
await escrowService.update(escrowId, {
  status: EscrowStatus.LOCKED,
});
// Esto automáticamente actualiza la orden a ONCHAIN_LOCKED
```

### 5. Actualizar Estado cuando se Liberan Fondos
```typescript
// Cuando se liberan fondos en blockchain
await escrowService.update(escrowId, {
  status: EscrowStatus.RELEASED,
  releaseTransactionHash: txHash,
});
```

## Validación de Consistencia

El sistema valida automáticamente:

### Datos
- ✅ Cantidad de crypto coincide entre orden y escrow
- ✅ Moneda coincide entre orden y escrow
- ✅ Existencia de registros

### Estados
- ✅ Orden `ONCHAIN_LOCKED` → Escrow `LOCKED`
- ✅ Orden `COMPLETED` → Escrow `RELEASED`
- ✅ Orden `REFUNDED` → Escrow `REFUNDED`

### Errores y Advertencias
- **Errores**: Inconsistencias críticas que deben corregirse
- **Advertencias**: Inconsistencias de estado que pueden ser temporales

## Sincronización con Orders

Cuando se actualiza el estado del escrow a `LOCKED`, el sistema automáticamente:
1. Actualiza el escrow con `lockedAt`
2. Actualiza la orden a estado `ONCHAIN_LOCKED`

Esto mantiene la sincronización entre ambos sistemas.

## Ejemplo de Uso Completo

```typescript
// 1. Orden creada y aceptada
const order = await ordersService.accept(orderId, buyerId);
// Estado: AWAITING_FUNDS

// 2. Crear escrow en blockchain (módulo blockchain)
const { escrowId, txHash } = await blockchainService.createEscrow({
  amount: order.cryptoAmount,
  currency: order.cryptoCurrency,
  seller: order.sellerId,
  buyer: order.buyerId,
});

// 3. Registrar mapeo
const escrow = await escrowService.create({
  orderId: order.id,
  escrowId,
  contractAddress: CONTRACT_ADDRESS,
  cryptoAmount: order.cryptoAmount,
  cryptoCurrency: order.cryptoCurrency,
  createTransactionHash: txHash,
});

// 4. Validar consistencia
const validation = await escrowService.validateConsistency(order.id);
console.log('Valid:', validation.isValid);

// 5. Cuando se bloquean fondos en blockchain
await escrowService.update(escrowId, {
  status: EscrowStatus.LOCKED,
});
// Orden automáticamente actualizada a ONCHAIN_LOCKED

// 6. Consultar mapeo
const mapping = await escrowService.getMapping(orderId: order.id);
console.log(`Order ${mapping.orderId} ↔ Escrow ${mapping.escrowId}`);
```

## Notas Importantes

1. **No ejecuta transacciones**: Este módulo solo registra y valida. Las transacciones blockchain se ejecutan en otro módulo.

2. **Mapeo único**: Cada orden tiene un único escrow y viceversa.

3. **Validación automática**: Se valida consistencia al crear y actualizar.

4. **Sincronización**: Los estados se sincronizan automáticamente entre orden y escrow.

5. **Errores de validación**: Se almacenan en `validationErrors` para debugging.

6. **Búsqueda bidireccional**: Puedes buscar por `orderId` o `escrowId`.
