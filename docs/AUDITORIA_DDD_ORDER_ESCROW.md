# Auditoría DDD: Order y Escrow — Consistencia transaccional

## 1. Modelo de dominio

### Agregado raíz: Order

- **Order** es el único agregado. Contiene la lógica de negocio y las invariantes.
- **Escrow** no existe ni cambia fuera del Order: es un valor/entidad interna; toda mutación de escrow pasa por el agregado.

### Value Object: Escrow

- **EscrowValueObject** (`app/domain/escrow_value_object.py`): objeto de valor inmutable que representa el estado del escrow dentro del Order.
- Estados de escrow: `PENDING`, `FUNDED`, `RELEASED`, `REFUNDED`.
- Transiciones legales: `PENDING → FUNDED`, `FUNDED → RELEASED`, `FUNDED → REFUNDED`.

### Invariantes de dominio (no deben romperse)

| Invariante | Garantía |
|------------|----------|
| Order no puede estar "pagado" sin escrow financiado | Estado `ESCROW_FUNDED` (order) solo si escrow está `FUNDED`. |
| Escrow no puede liberarse si el order no está RELEASED | Escrow `RELEASED` solo si order `RELEASED`. |
| Escrow no puede existir sin Order | Escrow siempre tiene `order_id`; se crea/actualiza solo desde el agregado. |
| Monto escrow = monto order | `apply_deposit` exige `amount`/`currency` igual al order; depósitos parciales se rechazan. |
| Depósitos procesados exactamente una vez | Idempotencia por clave (claim-first) + transacción con bloqueo de fila. |

---

## 2. Máquina de estados

### Order

```
CREATED ──(BUYER_ACCEPT)──► AWAITING_PAYMENT
    │                            │
    └──(CANCEL)──────────────────┼──(DEPOSIT_CONFIRMED / MANUAL_MARK_FUNDED)──► ESCROW_FUNDED
                                 │         │                                        │
                                 │         └──(CANCEL)──► CANCELLED                ├──(RELEASE)──► RELEASED
                                 │         └──(DISPUTE)──► DISPUTED                 ├──(CANCEL)──► CANCELLED
                                 └──(CANCEL)──► CANCELLED                            └──(DISPUTE)──► DISPUTED
                                                                                              │
                                                    DISPUTED ──(RESOLVE_RELEASE)──► RELEASED  │
                                                    DISPUTED ──(RESOLVE_REFUND)──► CANCELLED ◄┘
```

- **PAID**: equivalente lógico a `ESCROW_FUNDED` (order "pagado" cuando el escrow está financiado).
- Estados terminales: `RELEASED`, `CANCELLED`.

### Escrow

```
PENDING ──(deposit confirmado)──► FUNDED ──┬──(release)──► RELEASED
                                           └──(refund)──► REFUNDED
```

### Consistencia Order–Escrow

- `(order.ESCROW_FUNDED | DISPUTED)` ⇔ escrow `FUNDED`.
- `order.RELEASED` ⇔ escrow `RELEASED`.
- `order.CANCELLED` (con escrow) ⇔ escrow `REFUNDED`.

Las transiciones se validan en `OrderAggregate._validate_cross_invariants` y `_escrow_order_state_consistent`.

---

## 3. Cambios de esquema de base de datos

### Tablas existentes (sin cambio estructural)

- **orders**: `id`, `seller_id`, `buyer_id`, `crypto_*`, `fiat_*`, `status`, `escrow_id`, timestamps, etc.
- **escrows**: `id`, `order_id`, `external_escrow_id`, `contract_address`, `crypto_*`, `status`, `*_tx_hash`, timestamps.
- **idempotency_keys**: `id`, `idempotency_key` (UNIQUE), `order_id`, `event_type`, `result_snapshot`, `created_at`.
- **domain_events**: event log por order.

### Nueva tabla: outbox_events (Outbox)

- **005_outbox_events.py**: tabla `outbox_events` para publicación confiable de eventos.
- Columnas: `id`, `aggregate_type`, `aggregate_id`, `event_type`, `payload`, `created_at`, `processed_at`.
- Los eventos se escriben en la misma transacción que order/escrow; un worker marca `processed_at` al publicar.

### Migración de datos: 006_order_escrow_consistency

- Corrige divergencias históricas order/escrow:
  - Escrow `FUNDED` y order no `ESCROW_FUNDED`/`DISPUTED` → order a `ESCROW_FUNDED`.
  - Order `ESCROW_FUNDED` y escrow `PENDING` → escrow a `FUNDED`.
  - Order `RELEASED` y escrow no `RELEASED` → escrow a `RELEASED`.
  - Order `CANCELLED` con escrow no `REFUNDED` → escrow a `REFUNDED`.

---

## 4. Implementación del agregado Order

- **Archivo**: `app/domain/order_aggregate.py`.
- **Responsabilidades**:
  - Aceptar orden (`accept`), aplicar depósito (`apply_deposit`), adjuntar/crear escrow (`attach_escrow`, `link_escrow`), marcar financiado (`fund_escrow`, `record_escrow_locked`), completar (`complete`), reembolsar (`refund`), disputa y resolución (`open_dispute`, `resolve_dispute_release`, `resolve_dispute_refund`).
  - Validar FSM de order y consistencia order–escrow (`_ensure_fsm_ok`, `_validate_cross_invariants`, `_escrow_order_state_consistent`).
  - Emitir eventos de dominio; la persistencia (domain_events + outbox) se hace en la capa de aplicación en la misma transacción.

Ningún servicio ni API actualiza `EscrowModel` ni `OrderModel.status` directamente; todo pasa por el agregado.

---

## 5. Procesador de eventos de depósito

- **Archivo**: `app/services/deposit_processor.py`.
- **Flujo**:
  1. Calcular clave de idempotencia: `idempotency_key` del payload o `deposit:{order_id}:{tx_hash}`.
  2. **Claim idempotencia**: `INSERT` en `idempotency_keys`. Si falla por UNIQUE (duplicado), hacer rollback de la sesión, leer resultado cacheado y devolver `already_processed=True`.
  3. Cargar Order (y escrow) con `OrderRepository.get_for_update(order_id)` (bloqueo de fila).
  4. Si order ya está `ESCROW_FUNDED` con el mismo `tx_hash`, actualizar `result_snapshot` de la fila de idempotencia y hacer commit (replay idempotente).
  5. Si no: llamar `aggregate.apply_deposit(...)`, persistir agregado, persistir eventos (domain_events + outbox), actualizar `result_snapshot`, commit.
- Garantías: un solo apply por clave; depósitos duplicados o reintentos devuelven éxito sin reaplicar.

---

## 6. Mecanismo de idempotencia

- **Claim-first**: la primera operación que inserta la clave “gana”; el resto recibe conflicto y devuelve el resultado ya almacenado.
- Clave única en `idempotency_keys.idempotency_key`.
- `result_snapshot`: estado del order tras procesar (para respuestas cacheadas).
- Uso: webhooks de depósito en blockchain; cualquier evento que deba procesarse una sola vez puede reutilizar la misma tabla con otra `event_type`.

---

## 7. Flujo transaccional (depósito)

```
[Webhook] → process_deposit_event(db, payload)
    │
    ├─► INSERT idempotency_keys (key, order_id, event_type, result_snapshot=NULL)
    │       │
    │       ├─ IntegrityError → ROLLBACK → SELECT idempotency_keys → return DepositResult(already_processed=True)
    │       │
    │       └─ OK (flush, misma transacción)
    │
    ├─► get_for_update(order_id)  [bloqueo fila order + escrow]
    │
    ├─► Si order ESCROW_FUNDED y create_tx_hash == tx → actualizar result_snapshot, commit, return
    │
    ├─► aggregate.apply_deposit(...)  [solo si AWAITING_PAYMENT y montos correctos]
    ├─► repo.save(aggregate)
    ├─► persist_domain_events(db, events)  [domain_events + outbox_events]
    ├─► Actualizar idempotency_keys.result_snapshot
    └─► commit
```

Toda escritura de order/escrow y de eventos (incluido outbox) ocurre en una única transacción; el commit es atómico.

---

## 8. Tests

### Unitarios (`tests/test_order_aggregate.py`)

- Happy path: accept → fund_escrow → complete.
- Refund desde AWAITING_PAYMENT.
- Apertura de disputa.
- No adjuntar dos escrows; fund_escrow sin escrow exige datos.
- `get_for_update` devuelve agregado con order y escrow; None si order no existe.
- Resolución de disputa (release/refund) desde DISPUTED y desde ESCROW_FUNDED.
- Escrow y order solo se actualizan vía agregado (comprobación de código).
- create_escrow 404/409 y persistencia de domain events.
- `apply_deposit` correcto y rechazo de depósito parcial (monto/currency).
- Duplicado de webhook: misma clave procesada dos veces → una apply, ambas respuestas éxito.
- Replay mismo `tx_hash`: idempotente cuando order ya ESCROW_FUNDED con ese tx.
- Eventos escritos en outbox tras depósito.
- Invariante order–escrow: rechazo de par (order, escrow) inconsistente.

### Idempotencia e integración

- **test_concurrent_deposit_same_key_only_one_applies**: mismo payload procesado 5 veces con la misma clave de idempotencia; solo la primera aplica el depósito, las otras 4 devuelven `already_processed=True` (garantía claim-first).
- **test_integration_full_flow_create_accept_deposit_complete**: flujo completo create → accept → process_deposit → complete; order y escrow terminan en RELEASED sin divergencia.

---

## 9. Plan de migración

1. Aplicar migraciones en orden: hasta 004 (status rename), luego 005 (outbox), luego 006 (consistencia).
2. 006 corrige datos existentes para que order y escrow cumplan las invariantes.
3. Desplegar código que solo escribe escrow/order vía agregado y que usa `process_deposit_event` con claim idempotencia.
4. Opcional: worker que lea `outbox_events` donde `processed_at IS NULL`, publique el evento y actualice `processed_at`.

---

## 10. Escenarios de fallo y mitigación

| Escenario | Mitigación |
|-----------|------------|
| Webhook de depósito duplicado | Clave de idempotencia; segundo request devuelve resultado cacheado sin reaplicar. |
| Varias peticiones concurrentes mismo depósito | Claim-first: un solo INSERT exitoso; el resto recibe duplicado y devuelve cache. |
| Depósito parcial (monto/currency distinto) | `apply_deposit` rechaza con 409; order sigue en AWAITING_PAYMENT. |
| Order no encontrado | 404 antes de claim; si se hace claim y luego falla get_for_update, rollback (clave no queda comprometida si no hay commit). |
| Crash tras commit de eventos y antes de respuesta | Cliente puede reintentar con misma clave; respuesta idempotente. |
| Divergencia histórica order/escrow | Migración 006 alinea estados. |
| Escrow actualizado fuera del agregado | Prohibido por diseño; tests comprueban que servicios no escriben `EscrowModel.status` directamente. |

Con este diseño, Order y Escrow permanecen matemáticamente consistentes incluso con carga concurrente y webhooks repetidos.
