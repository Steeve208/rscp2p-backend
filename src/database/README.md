# Database Layer

Capa de base de datos con PostgreSQL y Redis.

## Estructura

```
database/
├── entities/        # Entidades TypeORM (PostgreSQL)
├── services/        # Servicios Redis (locks, sessions, rate limit)
├── migrations/     # Migraciones de base de datos
└── database.module.ts
```

## PostgreSQL

### Tablas Principales

#### users
- `id` (UUID)
- `wallet_address` (VARCHAR, UNIQUE, INDEXED)
- `reputation_score` (DECIMAL)
- `is_active` (BOOLEAN)
- `last_login_at` (TIMESTAMP)
- `login_count` (INTEGER)
- `created_at` (TIMESTAMP)
- `updated_at` (TIMESTAMP)

#### orders
- `id` (UUID)
- `seller_id` (UUID, INDEXED, FK → users)
- `buyer_id` (UUID, INDEXED, FK → users)
- `crypto_amount` (DECIMAL)
- `crypto_currency` (VARCHAR)
- `fiat_amount` (DECIMAL)
- `fiat_currency` (VARCHAR)
- `price_per_unit` (DECIMAL)
- `status` (ENUM, INDEXED)
- `escrow_id` (VARCHAR)
- `payment_method` (VARCHAR)
- `terms` (TEXT)
- `expires_at` (TIMESTAMP)
- `accepted_at` (TIMESTAMP)
- `completed_at` (TIMESTAMP)
- `cancelled_at` (TIMESTAMP)
- `cancelled_by` (VARCHAR)
- `created_at` (TIMESTAMP)
- `updated_at` (TIMESTAMP)

#### escrows
- `id` (UUID)
- `order_id` (UUID, UNIQUE, INDEXED, FK → orders)
- `escrow_id` (VARCHAR, UNIQUE, INDEXED)
- `contract_address` (VARCHAR)
- `create_transaction_hash` (VARCHAR)
- `crypto_amount` (DECIMAL)
- `crypto_currency` (VARCHAR)
- `status` (ENUM, INDEXED)
- `release_transaction_hash` (VARCHAR)
- `refund_transaction_hash` (VARCHAR)
- `locked_at` (TIMESTAMP)
- `released_at` (TIMESTAMP)
- `refunded_at` (TIMESTAMP)
- `validation_errors` (TEXT)
- `created_at` (TIMESTAMP)
- `updated_at` (TIMESTAMP)

#### disputes
- `id` (UUID)
- `order_id` (UUID, INDEXED, FK → orders)
- `initiator_id` (UUID, INDEXED, FK → users)
- `respondent_id` (UUID, FK → users)
- `reason` (TEXT)
- `status` (ENUM, INDEXED)
- `resolution` (TEXT)
- `resolved_by` (VARCHAR)
- `resolved_at` (TIMESTAMP)
- `escalated_at` (TIMESTAMP)
- `expires_at` (TIMESTAMP)
- `response_deadline` (TIMESTAMP)
- `evidence_deadline` (TIMESTAMP)
- `escrow_resolution` (VARCHAR)
- `escrow_resolved_at` (TIMESTAMP)
- `created_at` (TIMESTAMP)
- `updated_at` (TIMESTAMP)

#### dispute_evidence
- `id` (UUID)
- `dispute_id` (UUID, INDEXED, FK → disputes)
- `submitted_by` (UUID, INDEXED, FK → users)
- `evidence_type` (VARCHAR)
- `evidence_url` (TEXT)
- `description` (TEXT)
- `metadata` (TEXT)
- `created_at` (TIMESTAMP)

#### reputation_events
- `id` (UUID)
- `user_id` (UUID, INDEXED, FK → users)
- `event_type` (ENUM, INDEXED)
- `score_change` (DECIMAL)
- `order_id` (UUID, INDEXED)
- `dispute_id` (UUID, INDEXED)
- `reason` (TEXT)
- `metadata` (TEXT)
- `previous_score` (DECIMAL)
- `new_score` (DECIMAL)
- `created_at` (TIMESTAMP)

#### blockchain_events
- `id` (UUID)
- `event_name` (VARCHAR, INDEXED)
- `contract_address` (VARCHAR, INDEXED)
- `transaction_hash` (VARCHAR, UNIQUE, INDEXED)
- `block_number` (INTEGER, INDEXED)
- `block_hash` (VARCHAR)
- `event_data` (JSONB)
- `escrow_id` (VARCHAR, INDEXED)
- `order_id` (UUID, INDEXED)
- `processed` (BOOLEAN, INDEXED)
- `processed_at` (TIMESTAMP)
- `error_message` (TEXT)
- `created_at` (TIMESTAMP)

#### blockchain_sync
- `id` (UUID)
- `last_synced_block` (INTEGER)
- `last_synced_block_hash` (VARCHAR)
- `sync_status` (VARCHAR)
- `last_sync_at` (TIMESTAMP)
- `last_error` (TEXT)
- `total_events_processed` (INTEGER)
- `total_errors` (INTEGER)
- `created_at` (TIMESTAMP)
- `updated_at` (TIMESTAMP)

#### notifications
- `id` (UUID)
- `user_id` (UUID, INDEXED, FK → users)
- `type` (ENUM, INDEXED)
- `title` (TEXT)
- `message` (TEXT)
- `read` (BOOLEAN, INDEXED)
- `read_at` (TIMESTAMP)
- `data` (JSONB)
- `order_id` (UUID, INDEXED)
- `dispute_id` (UUID, INDEXED)
- `escrow_id` (VARCHAR, INDEXED)
- `created_at` (TIMESTAMP)

## Redis

### Uso de Redis

Redis se usa para:
1. **Locks de órdenes**: Prevenir operaciones concurrentes
2. **Rate limiting**: Control de velocidad de solicitudes
3. **Sesiones**: Almacenamiento de sesiones de usuario

### Servicios Redis

#### RedisLockService

Gestiona locks distribuidos para prevenir condiciones de carrera.

```typescript
import { RedisLockService } from '@/database/services';

// Adquirir lock
const acquired = await redisLockService.acquireOrderLock(orderId, 300);
if (!acquired) {
  throw new Error('Order is locked');
}

try {
  // Operación protegida
  await processOrder(orderId);
} finally {
  // Liberar lock
  await redisLockService.releaseOrderLock(orderId);
}

// O usar withLock (patrón try-finally automático)
await redisLockService.withLock(`order:${orderId}`, async () => {
  await processOrder(orderId);
});
```

**Métodos:**
- `acquireOrderLock(orderId, ttl)`: Adquiere lock de orden
- `releaseOrderLock(orderId)`: Libera lock
- `isOrderLocked(orderId)`: Verifica si está bloqueada
- `extendOrderLock(orderId, ttl)`: Extiende TTL
- `acquireLock(key, ttl)`: Lock genérico
- `releaseLock(key)`: Libera lock genérico
- `withLock(key, fn, ttl)`: Ejecuta función con lock

#### RedisSessionService

Gestiona sesiones de usuario.

```typescript
import { RedisSessionService } from '@/database/services';

// Crear sesión
await redisSessionService.setSession(sessionId, {
  userId: 'uuid',
  walletAddress: '0x...',
  accessToken: '...',
}, 86400); // 24 horas

// Obtener sesión
const session = await redisSessionService.getSession(sessionId);

// Extender sesión
await redisSessionService.extendSession(sessionId, 86400);

// Eliminar sesión
await redisSessionService.deleteSession(sessionId);
```

**Métodos:**
- `setSession(sessionId, data, ttl)`: Crea/actualiza sesión
- `getSession<T>(sessionId)`: Obtiene sesión
- `deleteSession(sessionId)`: Elimina sesión
- `extendSession(sessionId, ttl)`: Extiende TTL
- `sessionExists(sessionId)`: Verifica existencia
- `getSessionTtl(sessionId)`: Obtiene TTL restante
- `deleteUserSessions(userId)`: Elimina todas las sesiones de un usuario

#### RedisRateLimitService

Gestiona rate limiting.

```typescript
import { RedisRateLimitService } from '@/database/services';

// Verificar rate limit
const result = await redisRateLimitService.checkRateLimit(
  userId,
  10, // max requests
  60, // window seconds
);

if (!result.allowed) {
  throw new Error(`Rate limit exceeded. Try again at ${result.resetAt}`);
}

// Obtener información sin incrementar
const info = await redisRateLimitService.getRateLimitInfo(userId, 10);
console.log(`Remaining: ${info.remaining}`);
```

**Métodos:**
- `checkRateLimit(identifier, maxRequests, windowSeconds)`: Verifica e incrementa
- `resetRateLimit(identifier)`: Resetea contador
- `getRateLimitInfo(identifier, maxRequests)`: Obtiene info sin incrementar

## Configuración

### PostgreSQL

Variables de entorno:

```env
DB_HOST=localhost
DB_PORT=5432
DB_USERNAME=postgres
DB_PASSWORD=postgres
DB_DATABASE=rsc_db
```

### Redis

Variables de entorno:

```env
REDIS_HOST=localhost
REDIS_PORT=6379
REDIS_PASSWORD=
```

## Migraciones

### Generar Migración

```bash
npm run migration:generate -- -n MigrationName
```

### Ejecutar Migraciones

```bash
npm run migration:run
```

### Revertir Migración

```bash
npm run migration:revert
```

## Índices

### Índices Creados

- `users.wallet_address`: UNIQUE INDEX
- `orders.seller_id`: INDEX
- `orders.buyer_id`: INDEX
- `orders.status`: INDEX
- `escrows.order_id`: UNIQUE INDEX
- `escrows.escrow_id`: UNIQUE INDEX
- `escrows.status`: INDEX
- `disputes.order_id`: INDEX
- `disputes.initiator_id`: INDEX
- `disputes.status`: INDEX
- `notifications.user_id`: INDEX
- `notifications.type`: INDEX
- `notifications.read`: INDEX
- `blockchain_events.transaction_hash`: UNIQUE INDEX
- `blockchain_events.block_number`: INDEX
- `reputation_events.user_id`: INDEX
- `reputation_events.event_type`: INDEX

## Relaciones

### Foreign Keys

- `orders.seller_id` → `users.id`
- `orders.buyer_id` → `users.id`
- `escrows.order_id` → `orders.id`
- `disputes.order_id` → `orders.id`
- `disputes.initiator_id` → `users.id`
- `disputes.respondent_id` → `users.id`
- `dispute_evidence.dispute_id` → `disputes.id`
- `dispute_evidence.submitted_by` → `users.id`
- `reputation_events.user_id` → `users.id`
- `notifications.user_id` → `users.id`

## Uso de Locks

### Ejemplo: Procesar Orden con Lock

```typescript
import { RedisLockService } from '@/database/services';

async processOrder(orderId: string) {
  // Intentar adquirir lock
  const lockAcquired = await redisLockService.acquireOrderLock(orderId, 300);
  
  if (!lockAcquired) {
    throw new ConflictException('La orden está siendo procesada por otro proceso');
  }

  try {
    // Procesar orden
    const order = await orderRepository.findOne({ where: { id: orderId } });
    // ... lógica de procesamiento ...
  } finally {
    // Siempre liberar el lock
    await redisLockService.releaseOrderLock(orderId);
  }
}
```

### Ejemplo: Con withLock

```typescript
await redisLockService.withLock(`order:${orderId}`, async () => {
  // El lock se libera automáticamente al finalizar
  await processOrder(orderId);
});
```

## Uso de Sesiones

### Ejemplo: Gestión de Sesión

```typescript
import { RedisSessionService } from '@/database/services';

// Al autenticar
const sessionId = generateSessionId();
await redisSessionService.setSession(sessionId, {
  userId: user.id,
  walletAddress: user.walletAddress,
  accessToken: accessToken,
  refreshToken: refreshToken,
}, 86400);

// Al validar token
const session = await redisSessionService.getSession(sessionId);
if (!session) {
  throw new UnauthorizedException('Sesión expirada');
}

// Al refrescar
await redisSessionService.extendSession(sessionId, 86400);

// Al cerrar sesión
await redisSessionService.deleteSession(sessionId);
```

## Uso de Rate Limiting

### Ejemplo: Rate Limit en Endpoint

```typescript
import { RedisRateLimitService } from '@/database/services';

@Post('endpoint')
async endpoint(@Body() dto: any) {
  const identifier = request.user?.id || request.ip;
  const result = await redisRateLimitService.checkRateLimit(
    identifier,
    10, // 10 requests
    60, // por minuto
  );

  if (!result.allowed) {
    throw new HttpException(
      {
        statusCode: 429,
        message: `Rate limit exceeded. Try again at ${result.resetAt}`,
        resetAt: result.resetAt,
      },
      429,
    );
  }

  // Procesar solicitud
  return { remaining: result.remaining };
}
```

## Notas Importantes

1. **Locks**: Siempre liberar locks en finally o usar withLock
2. **Sesiones**: TTL automático, se eliminan al expirar
3. **Rate Limit**: Tolerante a fallos de Redis
4. **Índices**: Optimizados para búsquedas frecuentes
5. **Relaciones**: Foreign keys para integridad referencial
6. **Migrations**: Usar migraciones, no synchronize en producción
