# Módulo de Disputes

Sistema de gestión de conflictos humanos con evidencia off-chain, timers y resolución dependiente del escrow.

## Características

- ✅ Apertura de disputas
- ✅ Evidencia off-chain (imágenes, documentos, links, etc.)
- ✅ Timers y deadlines
- ✅ Gestión de estados
- ✅ **Resolución final siempre depende del escrow** (no decide fondos)
- ✅ Integración con reputation

## Principio Fundamental

**La resolución final siempre depende del escrow.** Este módulo:
- Gestiona el proceso de disputa off-chain
- Almacena evidencia
- Maneja timers y deadlines
- **NO decide fondos** - el escrow es quien resuelve

## Estados de Disputa

- **OPEN**: Disputa abierta, esperando evidencia
- **IN_REVIEW**: En revisión con evidencia presentada
- **RESOLVED**: Resuelta (resolución off-chain registrada)
- **CLOSED**: Cerrada (resolución del escrow aplicada)
- **ESCALATED**: Escalada (expiró sin resolución)

## Timers y Deadlines

- **Response Deadline**: 48 horas para responder
- **Evidence Deadline**: 72 horas para presentar evidencia
- **Escalation**: 7 días antes de escalar automáticamente

## Endpoints

### Abrir Disputa

```http
POST /api/disputes
Authorization: Bearer <token>
Content-Type: application/json

{
  "orderId": "uuid",
  "reason": "El vendedor no entregó el producto como se acordó"
}
```

**Reglas:**
- Solo el vendedor o comprador pueden abrir disputa
- Solo órdenes en estado `AWAITING_FUNDS` o `ONCHAIN_LOCKED`
- Solo una disputa abierta por orden
- Penalización de reputation: -15 puntos para ambas partes

**Respuesta:**
```json
{
  "id": "uuid",
  "orderId": "uuid",
  "initiatorId": "uuid",
  "respondentId": "uuid",
  "reason": "...",
  "status": "OPEN",
  "responseDeadline": "2024-01-03T00:00:00.000Z",
  "evidenceDeadline": "2024-01-04T00:00:00.000Z",
  "expiresAt": "2024-01-08T00:00:00.000Z",
  "createdAt": "2024-01-01T00:00:00.000Z"
}
```

### Listar Disputas

```http
GET /api/disputes?status=OPEN&orderId=xxx&userId=xxx
```

### Obtener Disputa

```http
GET /api/disputes/:id
```

**Respuesta incluye:**
- Información de la disputa
- Evidencia presentada
- Información de la orden
- Iniciador y respondiente

### Agregar Evidencia

```http
POST /api/disputes/:id/evidence
Authorization: Bearer <token>
Content-Type: application/json

{
  "evidenceType": "IMAGE",
  "evidenceUrl": "https://example.com/evidence.jpg",
  "description": "Screenshot del problema"
}
```

**Tipos de evidencia:**
- `IMAGE`: Imágenes
- `DOCUMENT`: Documentos
- `TEXT`: Texto
- `LINK`: Enlaces
- `VIDEO`: Videos
- `AUDIO`: Audio

**Reglas:**
- Solo partes de la disputa pueden agregar evidencia
- Solo antes del deadline de evidencia
- Cambia estado a `IN_REVIEW` automáticamente

### Resolver Disputa

```http
PUT /api/disputes/:id/resolve
Authorization: Bearer <token>
Content-Type: application/json

{
  "resolution": "Resolución off-chain registrada",
  "escrowResolution": "INITIATOR_WINS" // Viene del escrow
}
```

**IMPORTANTE:** La resolución final siempre viene del escrow. Este endpoint solo registra la resolución off-chain.

**Escrow Resolutions posibles:**
- `INITIATOR_WINS`: Gana el iniciador
- `RESPONDENT_WINS`: Gana el respondiente
- `SPLIT`: División de fondos
- `REFUND`: Reembolso completo

### Cerrar Disputa

```http
PUT /api/disputes/:id/close
Authorization: Bearer <token>
Content-Type: application/json

{
  "escrowResolution": "INITIATOR_WINS"
}
```

Cierra la disputa después de que el escrow resuelve los fondos.

### Escalar Disputa

```http
PUT /api/disputes/:id/escalate
Authorization: Bearer <token>
```

Escala una disputa que expiró sin resolución.

### Disputas Próximas a Expirar

```http
GET /api/disputes/expiring?hours=24
```

Obtiene disputas que expiran en las próximas N horas.

## Flujo Completo de una Disputa

### 1. Apertura de Disputa

```typescript
// Usuario abre disputa
const dispute = await disputesService.create(userId, {
  orderId: 'uuid',
  reason: 'Problema con la entrega',
});

// Automáticamente:
// - Orden cambia a DISPUTED
// - Escrow cambia a DISPUTED
// - Penalización de reputation para ambas partes
// - Se establecen deadlines
```

### 2. Presentación de Evidencia

```typescript
// Ambas partes pueden presentar evidencia
await disputesService.addEvidence(disputeId, userId, {
  evidenceType: 'IMAGE',
  evidenceUrl: 'https://...',
  description: 'Evidencia del problema',
});

// Estado cambia a IN_REVIEW automáticamente
```

### 3. Resolución del Escrow

```typescript
// El escrow resuelve en blockchain
// Este módulo recibe la resolución del escrow
await disputesService.resolve(disputeId, {
  resolution: 'Resolución registrada',
  escrowResolution: 'INITIATOR_WINS', // Viene del escrow
});

// Actualiza reputation según resultado
```

### 4. Cierre de Disputa

```typescript
// Después de que el escrow aplica la resolución
await disputesService.close(disputeId, 'INITIATOR_WINS');
```

## Integración con Reputation

El sistema actualiza automáticamente el reputation:

- **Abrir disputa**: -15 puntos (ambas partes)
- **Ganar disputa**: +10 puntos
- **Perder disputa**: -20 puntos

## Procesamiento de Timers

El sistema procesa timers automáticamente:

```typescript
// Procesar timers (ejecutar periódicamente)
const result = await disputesService.processTimers();
// Escala disputas expiradas
// Marca disputas con deadlines expirados
```

## Evidencia Off-Chain

La evidencia se almacena completamente off-chain:

- **URLs**: Enlaces a evidencia externa
- **Tipos**: Imágenes, documentos, videos, etc.
- **Metadata**: Información adicional
- **Tracking**: Quién presentó qué evidencia y cuándo

## Resolución Dependiente del Escrow

**CRÍTICO:** Este módulo NO decide fondos. El flujo es:

1. **Disputa off-chain**: Gestiona el proceso, evidencia, timers
2. **Resolución del escrow**: El escrow (blockchain) decide los fondos
3. **Registro off-chain**: Este módulo registra la resolución del escrow
4. **Actualización de reputation**: Basado en la resolución del escrow

El escrow es la fuente de verdad para la resolución de fondos.

## Ejemplo de Uso Completo

```typescript
// 1. Abrir disputa
const dispute = await fetch('/api/disputes', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    orderId: 'uuid',
    reason: 'Producto no entregado',
  }),
}).then(r => r.json());

// 2. Agregar evidencia
await fetch(`/api/disputes/${dispute.id}/evidence`, {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    evidenceType: 'IMAGE',
    evidenceUrl: 'https://example.com/evidence.jpg',
    description: 'Screenshot del problema',
  }),
});

// 3. Escrow resuelve (en blockchain)
// El escrow decide: INITIATOR_WINS

// 4. Registrar resolución
await fetch(`/api/disputes/${dispute.id}/resolve`, {
  method: 'PUT',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    resolution: 'Resolución registrada',
    escrowResolution: 'INITIATOR_WINS',
  }),
});

// 5. Cerrar disputa
await fetch(`/api/disputes/${dispute.id}/close`, {
  method: 'PUT',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    escrowResolution: 'INITIATOR_WINS',
  }),
});
```

## Notas Importantes

1. **No decide fondos**: Este módulo solo gestiona el proceso off-chain
2. **Escrow es la verdad**: La resolución final siempre viene del escrow
3. **Evidencia off-chain**: Toda la evidencia se almacena off-chain
4. **Timers automáticos**: Los deadlines se procesan automáticamente
5. **Reputation automática**: Se actualiza según resultados del escrow
6. **Una disputa por orden**: Solo una disputa abierta por orden a la vez
