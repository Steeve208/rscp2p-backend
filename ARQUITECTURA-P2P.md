# Arquitectura P2P Wallet-to-Wallet - RSC Finance

## 🎯 Visión General

RSC Finance es una plataforma **peer-to-peer** donde los usuarios operan directamente **wallet-to-wallet**, sin intermediarios tradicionales.

## 🔐 Autenticación Wallet-Based

### Flujo de Autenticación

```
1. Usuario conecta wallet (MetaMask, WalletConnect, etc.)
   ↓
2. Frontend solicita challenge: POST /api/auth/challenge
   Body: { walletAddress: "0x..." }
   ↓
3. Backend genera nonce único y mensaje firmable
   Response: { nonce: "...", message: "Bienvenido a RSC Finance..." }
   ↓
4. Usuario firma el mensaje con su wallet privada
   ↓
5. Frontend envía firma: POST /api/auth/verify
   Body: { walletAddress: "0x...", nonce: "...", signature: "0x..." }
   ↓
6. Backend verifica firma criptográficamente
   ↓
7. Backend crea/actualiza usuario en DB (por wallet_address)
   ↓
8. Backend emite tokens JWT (access + refresh)
   ↓
9. Usuario autenticado puede operar en la plataforma
```

### Características de Seguridad

- ✅ **Sin passwords**: Solo firmas criptográficas
- ✅ **Nonces únicos**: Cada challenge es de un solo uso
- ✅ **TTL de nonces**: 5 minutos de validez
- ✅ **Rate limiting**: 10 challenges/min, 5 verificaciones/min por wallet
- ✅ **Verificación criptográfica**: Usa `ethers.verifyMessage()`

## 👥 Sistema de Usuarios

### Modelo de Datos

```typescript
User {
  id: UUID                    // ID interno
  walletAddress: string       // Dirección Ethereum (ÚNICA, INDEXED)
  reputationScore: decimal    // Puntuación de reputación
  isActive: boolean           // Estado del usuario
  lastLoginAt: Date          // Último login
  loginCount: number          // Contador de logins
  createdAt: Date
  updatedAt: Date
}
```

### Características

- **Pseudónimo**: Solo wallet address, sin información personal
- **Auto-creación**: Usuario se crea automáticamente en primer login
- **Búsqueda pública**: Cualquiera puede buscar usuarios por wallet address
- **Reputación off-chain**: Sistema de confianza basado en transacciones

## 💼 Transacciones P2P

### Flujo de una Orden P2P

```
1. Usuario A crea orden: "Vendo 100 USDT por 0.05 ETH"
   ↓
2. Sistema crea escrow on-chain (smart contract)
   ↓
3. Usuario B acepta la orden
   ↓
4. Usuario B deposita fondos en escrow
   ↓
5. Sistema escucha evento on-chain: "FundsDeposited"
   ↓
6. Sistema actualiza estado en DB: order.status = "funded"
   ↓
7. Usuario A entrega el servicio/producto
   ↓
8. Usuario B confirma recepción
   ↓
9. Sistema libera fondos del escrow on-chain
   ↓
10. Sistema actualiza reputación de ambos usuarios
```

### Componentes

- **Orders**: Órdenes P2P (compra/venta)
- **Escrows**: Mapeo order_id ↔ escrow_id on-chain
- **Blockchain Events**: Escucha y reconciliación de estados
- **Reputation**: Sistema de confianza basado en transacciones completadas

## 🗄️ Arquitectura de Datos

### PostgreSQL (Base de Datos Principal)

**Tablas principales:**
- `users`: Usuarios identificados por wallet_address
- `orders`: Órdenes P2P entre usuarios
- `escrows`: Mapeo de órdenes a contratos on-chain
- `disputes`: Disputas entre usuarios
- `reputation_events`: Eventos que afectan la reputación
- `notifications`: Notificaciones para usuarios
- `blockchain_events`: Eventos escuchados de la blockchain
- `blockchain_sync`: Estado de sincronización con blockchain

**Características:**
- Índices únicos en `wallet_address`
- Relaciones entre órdenes, usuarios, escrows
- Auditoría de cambios (created_at, updated_at)

### Redis (Cache y Sesiones)

**Uso principal:**
- `auth:session:{userId}`: Sesiones JWT (refresh tokens)
- `auth:nonce:{walletAddress}:{nonce}`: Nonces temporales (TTL 5 min)
- `auth:ratelimit:{walletAddress}:{action}`: Rate limiting por wallet
- `lock:{resource}`: Locks distribuidos para operaciones críticas
- `session:{sessionId}`: Sesiones generales (si se necesitan)

**TTLs:**
- Sesiones JWT: 7 días (refresh token)
- Nonces: 5 minutos
- Rate limiting: 1 minuto (ventana deslizante)

## 🔄 Sincronización Blockchain

### Eventos Escuchados

El sistema escucha eventos del smart contract de escrow:
- `EscrowCreated`: Nuevo escrow creado
- `FundsDeposited`: Fondos depositados en escrow
- `FundsReleased`: Fondos liberados
- `EscrowCancelled`: Escrow cancelado

### Reconciliación

- El backend escucha eventos on-chain
- Actualiza estados en PostgreSQL
- Notifica a usuarios vía WebSocket
- Maneja discrepancias entre on-chain y off-chain

## 🔌 WebSocket (Tiempo Real)

### Eventos Emitidos

- `order:created`: Nueva orden creada
- `order:updated`: Orden actualizada
- `order:accepted`: Orden aceptada
- `notification`: Nueva notificación
- `dispute:created`: Nueva disputa

### Autenticación WebSocket

- Usuarios se conectan con su JWT token
- El gateway valida el token
- Asocia `walletAddress` al socket
- Filtra eventos por wallet del usuario

## 🛡️ Seguridad y Rate Limiting

### Rate Limiting por Wallet

- **Challenge**: 10 solicitudes por minuto por wallet
- **Verify**: 5 solicitudes por minuto por wallet
- **API general**: 100 requests por minuto por IP

### Validaciones

- Validación de direcciones Ethereum
- Verificación de firmas criptográficas
- Sanitización de inputs
- Circuit breakers para servicios externos

## 📊 Sistema de Reputación

### Cálculo Off-Chain

- Basado en transacciones completadas
- Penalizaciones por disputas
- Bonificaciones por transacciones exitosas
- Historial completo en `reputation_events`

### Uso

- Búsqueda de usuarios por reputación
- Filtrado de órdenes por reputación mínima
- Visualización pública de puntuación

## 🚀 Despliegue en Producción

### Requisitos

1. **PostgreSQL**: Base de datos principal
2. **Redis**: Sesiones y rate limiting (crítico)
3. **Node.js**: Backend NestJS
4. **PM2**: Gestión de procesos
5. **Blockchain RPC**: Conexión a red Ethereum

### Variables de Entorno Críticas

```env
# Base de datos
DB_HOST=localhost
DB_USERNAME=rsc_user
DB_PASSWORD=...
DB_DATABASE=rsc_db

# Redis (CRÍTICO para autenticación)
REDIS_HOST=localhost
REDIS_PORT=6379
REDIS_PASSWORD=...

# JWT
JWT_SECRET=... (mínimo 32 caracteres)

# Blockchain
BLOCKCHAIN_RPC_URL=https://...
ESCROW_CONTRACT_ADDRESS=0x...
```

### Sin Redis = Sin Autenticación

⚠️ **IMPORTANTE**: Si Redis no está disponible:
- Los usuarios NO podrán autenticarse
- No se pueden generar challenges
- No se pueden verificar firmas
- No hay rate limiting

## 📝 Resumen

- ✅ **P2P**: Transacciones directas wallet-to-wallet
- ✅ **Sin passwords**: Solo firmas criptográficas
- ✅ **Pseudónimo**: Solo wallet addresses
- ✅ **On-chain + Off-chain**: Escrows on-chain, reputación off-chain
- ✅ **Tiempo real**: WebSocket para notificaciones
- ✅ **Seguro**: Rate limiting, validaciones, circuit breakers

