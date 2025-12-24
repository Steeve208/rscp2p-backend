# Módulo de Blockchain - El Oyente de la Verdad

Sistema completo de escucha, validación y reconciliación de eventos blockchain.

## Características

- ✅ **Listeners de eventos**: Escucha eventos en tiempo real del contrato escrow
- ✅ **Validadores de bloques**: Valida integridad de bloques y cadenas
- ✅ **Reconciliación de estados**: Sincroniza estados off-chain con on-chain
- ✅ **Re-sincronización**: Puede re-sincronizarse si el servicio se cae
- ✅ **Procesamiento de eventos**: Procesa eventos y actualiza estados automáticamente

## Arquitectura

### Componentes Principales

1. **EventListenerService**: Escucha eventos blockchain en tiempo real
2. **BlockValidatorService**: Valida bloques y cadenas
3. **StateReconcilerService**: Reconcilia estados entre blockchain y base de datos
4. **SyncService**: Gestiona sincronización y re-sincronización
5. **BlockchainService**: Servicio principal que orquesta todo

### Entidades

- **BlockchainEvent**: Almacena eventos recibidos de la blockchain
- **BlockchainSync**: Trackea el estado de sincronización

## Eventos Escuchados

El sistema escucha los siguientes eventos del contrato escrow:

- `EscrowCreated`: Escrow creado
- `FundsLocked`: Fondos bloqueados
- `FundsReleased`: Fondos liberados
- `FundsRefunded`: Fondos reembolsados
- `DisputeOpened`: Disputa abierta

## Endpoints

### Estado de Blockchain

```http
GET /api/blockchain/status
```

**Respuesta:**
```json
{
  "status": "connected",
  "network": "mainnet",
  "latestBlock": 18500000,
  "syncStatus": "ACTIVE",
  "lastSyncedBlock": 18499950,
  "totalEventsProcessed": 1234,
  "totalErrors": 5
}
```

### Iniciar Sincronización

```http
POST /api/blockchain/sync/start
```

### Detener Sincronización

```http
POST /api/blockchain/sync/stop
```

### Re-sincronizar desde Bloque

```http
POST /api/blockchain/sync/resync/:blockNumber
```

### Re-sincronización Automática

```http
POST /api/blockchain/sync/auto-resync
```

### Reconcilia Todos los Estados

```http
POST /api/blockchain/reconcile/all
```

**Respuesta:**
```json
{
  "total": 50,
  "reconciled": 48,
  "errors": 2
}
```

### Reconcilia Escrow Específico

```http
POST /api/blockchain/reconcile/escrow/:escrowId
```

**Respuesta:**
```json
{
  "reconciled": true,
  "changes": [
    "Escrow 0x... locked",
    "Order uuid completed"
  ]
}
```

### Validar Bloque

```http
GET /api/blockchain/validate/block/:blockNumber
```

**Respuesta:**
```json
{
  "isValid": true,
  "block": {
    "number": 18500000,
    "hash": "0x...",
    "timestamp": 1234567890
  },
  "errors": []
}
```

### Último Bloque

```http
GET /api/blockchain/latest-block
```

### Balance de Dirección

```http
GET /api/blockchain/balance/:address
```

## Flujo de Sincronización

### 1. Inicio de Sincronización

```typescript
// Al iniciar la aplicación
await blockchainService.startSync();
```

Esto:
- Inicia listeners de eventos en tiempo real
- Sincroniza desde el último bloque procesado
- Valida la cadena de bloques
- Procesa eventos pendientes

### 2. Escucha de Eventos en Tiempo Real

Los eventos se escuchan automáticamente y se guardan en la base de datos:

```typescript
// Evento recibido automáticamente
{
  eventName: "FundsLocked",
  escrowId: "0x...",
  transactionHash: "0x...",
  blockNumber: 18500000,
  processed: false
}
```

### 3. Procesamiento de Eventos

Los eventos se procesan automáticamente:

- **FundsLocked** → Actualiza escrow a `LOCKED`, orden a `ONCHAIN_LOCKED`
- **FundsReleased** → Actualiza escrow a `RELEASED`, orden a `COMPLETED`
- **FundsRefunded** → Actualiza escrow a `REFUNDED`, orden a `REFUNDED`
- **DisputeOpened** → Actualiza escrow a `DISPUTED`, orden a `DISPUTED`

### 4. Reconciliación Periódica

El job ejecuta reconciliación cada minuto:

```typescript
@Cron(CronExpression.EVERY_MINUTE)
async handleBlockchainSync() {
  await blockchainService.autoResyncIfNeeded();
  await blockchainService.reconcileAll();
}
```

## Re-sincronización

### Cuándo se Activa

La re-sincronización automática se activa cuando:

1. El estado de sincronización es `ERROR`
2. No se ha sincronizado en más de 1 hora
3. Se solicita manualmente

### Proceso de Re-sincronización

```typescript
// Re-sincronizar desde un bloque específico
await blockchainService.resyncFromBlock(18400000);

// Esto:
// 1. Valida la cadena desde ese bloque
// 2. Escanea todos los eventos históricos
// 3. Procesa eventos no procesados
// 4. Reconcilia todos los estados
```

### Re-sincronización Automática

```typescript
// Se ejecuta automáticamente si detecta problemas
await blockchainService.autoResyncIfNeeded();
```

## Validación de Bloques

### Validación Individual

```typescript
const validation = await blockchainService.validateBlock(18500000);
// Valida:
// - Existencia del bloque
// - Hash del bloque
// - Número del bloque
// - Timestamp
// - Hash del bloque padre
```

### Validación de Cadena

```typescript
const validation = await blockValidator.validateBlockChain(18400000, 18500000);
// Valida:
// - Todos los bloques en el rango
// - Integridad de la cadena (parent hashes)
```

## Reconciliación de Estados

### Reconciliación de Escrow

```typescript
const result = await blockchainService.reconcileEscrow('0x...');
// Procesa eventos pendientes del escrow
// Actualiza estados de escrow y orden
// Valida consistencia
```

### Reconciliación Masiva

```typescript
const result = await blockchainService.reconcileAll();
// Reconcilia todos los escrows pendientes
// Procesa todos los eventos no procesados
```

## Jobs Automáticos

### Sincronización Continua (Cada Minuto)

- Re-sincroniza automáticamente si es necesario
- Reconcilia eventos no procesados
- Actualiza estados

### Verificación de Estado (Cada 5 Minutos)

- Verifica estado de conexión
- Reporta errores
- Muestra estadísticas

### Reconciliación Profunda (Cada Hora)

- Reconciliación completa de todos los escrows
- Procesa eventos pendientes
- Valida consistencia global

## Manejo de Errores

### Errores de Conexión

Si se pierde la conexión:
1. El estado cambia a `ERROR`
2. Se registra el error
3. La re-sincronización automática intenta recuperarse

### Errores de Procesamiento

Si un evento falla al procesarse:
1. Se marca con `errorMessage`
2. Se mantiene como `processed: false`
3. Se reintenta en la próxima reconciliación

### Re-sincronización después de Caída

Si el servicio se cae:
1. Al reiniciar, detecta el último bloque sincronizado
2. Re-sincroniza desde ese bloque
3. Procesa todos los eventos perdidos

## Configuración

Variables de entorno necesarias:

```env
BLOCKCHAIN_RPC_URL=https://eth.llamarpc.com
BLOCKCHAIN_NETWORK=mainnet
ESCROW_CONTRACT_ADDRESS=0x...
BLOCKCHAIN_PRIVATE_KEY=0x... (opcional)
```

## Ejemplo de Uso Completo

```typescript
// 1. Iniciar sincronización
await blockchainService.startSync();

// 2. Verificar estado
const status = await blockchainService.getStatus();
console.log(`Sync status: ${status.syncStatus}`);

// 3. Reconciliar un escrow específico
const result = await blockchainService.reconcileEscrow('0x...');
console.log(`Reconciled: ${result.reconciled}, Changes: ${result.changes}`);

// 4. Re-sincronizar si hay problemas
if (status.syncStatus === 'ERROR') {
  await blockchainService.resyncFromBlock(status.lastSyncedBlock);
}

// 5. Validar un bloque
const validation = await blockchainService.validateBlock(18500000);
console.log(`Block valid: ${validation.isValid}`);
```

## Notas Importantes

1. **El oyente de la verdad**: Este módulo es la fuente de verdad para estados blockchain
2. **Re-sincronización**: Puede recuperarse automáticamente después de caídas
3. **Validación continua**: Valida bloques y cadenas constantemente
4. **Reconciliación automática**: Sincroniza estados off-chain con on-chain
5. **Procesamiento idempotente**: Los eventos se procesan solo una vez
6. **Tolerancia a fallos**: Continúa funcionando aunque algunos eventos fallen
