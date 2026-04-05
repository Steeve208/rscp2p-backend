# Idempotencia en mutaciones de órdenes

## Objetivo

Evitar ejecuciones duplicadas por reintentos de red o doble clic en los endpoints de mutación de órdenes. Cada petición con el mismo `Idempotency-Key` debe devolver el resultado almacenado sin volver a ejecutar el comando.

## Header requerido

Todas las mutaciones de orden deben enviar:

```http
Idempotency-Key: <uuid>
```

Ejemplo: `Idempotency-Key: 550e8400-e29b-41d4-a716-446655440000`

Si falta el header o no es un UUID válido, el backend responde `400 Bad Request`.

## Endpoints afectados

| Método | Ruta | Descripción |
|--------|------|-------------|
| PUT | /api/orders/{id}/accept | Aceptar oferta (comprador) |
| PUT | /api/orders/{id}/mark-locked | Marcar fondos bloqueados |
| PUT | /api/orders/{id}/complete | Completar orden (liberar) |
| PUT | /api/orders/{id}/cancel | Cancelar orden |
| PUT | /api/orders/{id}/dispute | Abrir disputa |

## Tabla `idempotency_keys`

- **key** (idempotency_key): UUID enviado en el header (único).
- **endpoint**: Identificador del endpoint (ej. `orders/accept`).
- **order_id**: ID de la orden.
- **response_status**: Código HTTP de la respuesta almacenada (200, 404, 409, etc.).
- **result_snapshot**: Cuerpo de la respuesta en JSON (Order o `{"detail": "..."}`).
- **response_hash**: Hash opcional del snapshot para verificación.
- **created_at**: Fecha de la primera petición.

La tabla ya existía (migración 003); la migración **009** añade `response_status`, `response_hash` y `endpoint`.

## Flujo

1. **Primera petición con clave K**
   - `INSERT` en `idempotency_keys` (key K, endpoint, order_id, response_status=NULL).
   - Si el `INSERT` tiene éxito, se ejecuta el comando (accept/cancel/complete/etc.).
   - Se actualiza la fila con `response_status` y `result_snapshot` (y opcionalmente `response_hash`).
   - Se devuelve la respuesta al cliente.

2. **Petición duplicada (misma clave K)**
   - El `INSERT` falla por restricción UNIQUE.
   - Se hace `SELECT` por `idempotency_key = K`.
   - Si `response_status` no es NULL: se devuelve `(response_status, result_snapshot)` sin ejecutar el comando.
   - Si `response_status` es NULL (primera petición aún en curso): se devuelve `409 Conflict` con detalle "Idempotent request in progress; retry after a short delay".

3. **Seguridad transaccional**
   - Solo una petición puede “adjudicarse” la clave (UNIQUE en `idempotency_key`).
   - No se producen transiciones de estado duplicadas, ni operaciones de escrow ni entradas de ledger duplicadas.

## Implementación

- **Dependencia**: `app.api.deps.require_idempotency_key` — lee el header y valida formato UUID.
- **Servicio**: `app.services.idempotency_service.run_idempotent` — reclama clave, ejecuta el callable, guarda resultado y devuelve `(status_code, body_dict)`.
- **Rutas**: En `app.api.routes.orders` los cinco PUT usan `Depends(require_idempotency_key)` y devuelven `JSONResponse(content=content, status_code=status)`.

## Tests

- `tests/test_idempotency_orders.py`: misma clave devuelve resultado cacheado; claves distintas generan dos filas y cada una guarda su resultado.
