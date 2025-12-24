# Módulo de Notifications

Sistema de notificaciones con WebSocket para eventos de mercado y cambios de estado.

## Características

- ✅ **WebSocket**: Notificaciones en tiempo real
- ✅ **Eventos de mercado**: Actualizaciones de precios, órdenes, etc.
- ✅ **Cambios de estado**: Notificaciones de cambios en órdenes, disputas, escrows
- ✅ **No lógica crítica**: Solo notificaciones, no afecta operaciones
- ✅ **Almacenamiento**: Notificaciones guardadas en base de datos
- ✅ **Múltiples canales**: Suscripción a diferentes tipos de eventos

## Tipos de Notificaciones

- `ORDER_CREATED`: Orden creada
- `ORDER_ACCEPTED`: Orden aceptada
- `ORDER_COMPLETED`: Orden completada
- `ORDER_CANCELLED`: Orden cancelada
- `ORDER_DISPUTED`: Orden disputada
- `DISPUTE_OPENED`: Disputa abierta
- `DISPUTE_RESOLVED`: Disputa resuelta
- `ESCROW_LOCKED`: Fondos bloqueados
- `ESCROW_RELEASED`: Fondos liberados
- `ESCROW_REFUNDED`: Fondos reembolsados
- `MARKET_UPDATE`: Actualización de mercado
- `PRICE_UPDATE`: Actualización de precio
- `REPUTATION_CHANGE`: Cambio de reputación

## WebSocket

### Conexión

```javascript
import io from 'socket.io-client';

const socket = io('http://localhost:3000/market', {
  auth: {
    token: 'your-jwt-token', // Opcional
  },
});
```

### Suscripciones

#### Suscribirse a Canal General

```javascript
socket.emit('subscribe', { channel: 'market:updates' });
socket.on('market:update', (data) => {
  console.log('Market update:', data);
});
```

#### Suscribirse a Notificaciones de Usuario

```javascript
// Requiere autenticación
socket.emit('subscribe:user');
socket.on('notification', (data) => {
  console.log('Notification:', data.notification);
});
```

#### Suscribirse a Orden Específica

```javascript
socket.emit('subscribe', { channel: 'order:uuid' });
socket.on('order:update', (data) => {
  console.log('Order update:', data);
});
```

#### Suscribirse a Precios

```javascript
socket.emit('subscribe', { channel: 'price:BTC' });
socket.on('price:update', (data) => {
  console.log('Price update:', data);
});
```

### Eventos Emitidos

#### Notificaciones de Usuario

```javascript
socket.on('notification', (data) => {
  // {
  //   type: 'notification',
  //   notification: {
  //     id: 'uuid',
  //     type: 'ORDER_COMPLETED',
  //     title: 'Orden completada',
  //     message: '...',
  //     data: {...}
  //   }
  // }
});
```

#### Actualizaciones de Orden

```javascript
socket.on('order:update', (data) => {
  // {
  //   orderId: 'uuid',
  //   status: 'COMPLETED',
  //   ...
  // }
});
```

#### Cambios de Estado

```javascript
socket.on('status:change', (data) => {
  // {
  //   entity: 'order',
  //   entityId: 'uuid',
  //   status: 'COMPLETED',
  //   data: {...}
  // }
});
```

#### Actualizaciones de Mercado

```javascript
socket.on('market:update', (data) => {
  // Actualización general del mercado
});
```

#### Actualizaciones de Precio

```javascript
socket.on('price:update', (data) => {
  // {
  //   symbol: 'BTC',
  //   price: 50000
  // }
});
```

## Endpoints HTTP

### Obtener Notificaciones

```http
GET /api/notifications?limit=50&unreadOnly=false
Authorization: Bearer <token>
```

**Respuesta:**
```json
[
  {
    "id": "uuid",
    "type": "ORDER_COMPLETED",
    "title": "Orden completada",
    "message": "Tu orden ha sido completada",
    "read": false,
    "data": {
      "orderId": "uuid",
      "status": "COMPLETED"
    },
    "createdAt": "2024-01-01T00:00:00.000Z"
  }
]
```

### Conteo de No Leídas

```http
GET /api/notifications/unread-count
Authorization: Bearer <token>
```

**Respuesta:**
```json
{
  "count": 5
}
```

### Marcar como Leída

```http
PUT /api/notifications/:id/read
Authorization: Bearer <token>
```

### Marcar Todas como Leídas

```http
PUT /api/notifications/read-all
Authorization: Bearer <token>
```

## Uso en Otros Módulos

### Notificar Cambio de Estado de Orden

```typescript
import { NotificationsService } from '../notifications/notifications.service';

// En OrdersService
await this.notificationsService.notifyOrderStatusChange(
  userId,
  orderId,
  'COMPLETED',
  { orderData }
);
```

### Notificar Apertura de Disputa

```typescript
await this.notificationsService.notifyDisputeOpened(
  userId,
  disputeId,
  orderId,
);
```

### Notificar Cambio de Escrow

```typescript
await this.notificationsService.notifyEscrowStatusChange(
  userId,
  escrowId,
  'RELEASED',
  { escrowData }
);
```

### Notificar Actualización de Mercado

```typescript
await this.notificationsService.notifyMarketUpdate(userId, {
  message: 'Nueva oferta disponible',
  data: {...}
});
```

### Notificar Cambio de Reputation

```typescript
await this.notificationsService.notifyReputationChange(
  userId,
  newScore,
  change,
);
```

## Canales WebSocket

### Canales Disponibles

- `market:updates` - Actualizaciones generales del mercado
- `user:{userId}` - Notificaciones del usuario (requiere auth)
- `order:{orderId}` - Actualizaciones de orden específica
- `dispute:{disputeId}` - Eventos de disputa
- `escrow:{escrowId}` - Eventos de escrow
- `price:{symbol}` - Actualizaciones de precio

### Ejemplo Completo

```javascript
const socket = io('http://localhost:3000/market', {
  auth: { token: jwtToken },
});

// Conectar
socket.on('connect', () => {
  console.log('Connected');

  // Suscribirse a notificaciones de usuario
  socket.emit('subscribe:user');

  // Suscribirse a mercado
  socket.emit('subscribe', { channel: 'market:updates' });

  // Suscribirse a orden específica
  socket.emit('subscribe', { channel: 'order:uuid' });
});

// Escuchar notificaciones
socket.on('notification', (data) => {
  const notification = data.notification;
  console.log(`New notification: ${notification.title}`);
  console.log(notification.message);
});

// Escuchar actualizaciones de orden
socket.on('order:update', (data) => {
  console.log(`Order ${data.orderId} updated: ${data.status}`);
});

// Escuchar cambios de estado
socket.on('status:change', (data) => {
  console.log(`${data.entity} ${data.entityId} changed to ${data.status}`);
});

// Desconectar
socket.on('disconnect', () => {
  console.log('Disconnected');
});
```

## Integración Automática

El sistema emite notificaciones automáticamente cuando:

- Se crea/acepta/completa/cancela una orden
- Se abre/resuelve una disputa
- Cambia el estado de un escrow
- Cambia la reputación de un usuario
- Hay actualizaciones de mercado

## Notas Importantes

1. **No lógica crítica**: Las notificaciones no afectan operaciones
2. **Tolerancia a fallos**: Si WebSocket falla, las notificaciones se guardan en BD
3. **Autenticación opcional**: WebSocket funciona sin auth, pero algunas funciones requieren token
4. **Múltiples clientes**: Un usuario puede tener múltiples conexiones WebSocket
5. **Persistencia**: Todas las notificaciones se guardan en base de datos
6. **Limpieza automática**: Notificaciones antiguas se eliminan automáticamente
