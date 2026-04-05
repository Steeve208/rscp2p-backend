# Diseño: Order como Aggregate Root (DDD)

Refactorización para que Order sea el Aggregate Root y Escrow sea entidad interna. Sin cambios en contratos HTTP, rutas FastAPI ni esquemas Pydantic.

---

## A) Nueva arquitectura de carpetas

```
app/
  api/routes/
    orders.py          # Sin cambios (misma API)
    escrow.py          # Sin cambios (misma API)
  domain/
    order_state_machine.py   # Sin cambios
    order_aggregate.py       # NUEVO: OrderAggregate + métodos de dominio
  models/
    marketplace.py          # Sin cambios (OrderModel, EscrowModel)
  repositories/
    order_repository.py     # NUEVO: OrderRepository (get_for_update, save)
  services/
    orders.py               # Refactor: usa repository + aggregate, no toca status/escrow directo
    escrow.py               # Refactor: creación/actualización vía aggregate; lecturas igual
  schemas/
    order.py                # Sin cambios
    escrow.py               # Sin cambios
tests/
  test_order_aggregate.py   # NUEVO: flujo, invariantes, concurrencia, doble escrow
  test_order_state_machine.py  # Sin cambios
```

- **domain/order_aggregate.py**: raíz del agregado; contiene `order` (OrderModel) y `escrow` (EscrowModel | None); métodos de dominio sin commit ni acceso a db.
- **repositories/order_repository.py**: carga con `SELECT FOR UPDATE` (order + escrow) y devuelve `OrderAggregate`; `save(aggregate)` solo persiste en la sesión (commit fuera).
- **services**: orquestan: obtienen agregado vía repositorio, llaman método del agregado, luego `repository.save(aggregate)` y `db.commit()`. No modifican `OrderModel.status` ni `EscrowModel.status` directamente.
- Rutas y esquemas permanecen igual; la compatibilidad HTTP se mantiene por la capa de servicios que siguen devolviendo los mismos DTOs (Order, Escrow).

---

## B) Cómo Order se convierte en Aggregate Root

- **Antes:** Servicios y FSM actuaban sobre `OrderModel` y, por separado, `EscrowModel`; `create_escrow` / `update_escrow` tocaban escrow y a veces la orden; `set_order_escrow` asignaba `order.escrow_id` fuera de un flujo coherente.
- **Después:** Toda la lógica que cambia estado de orden o escrow vive en **OrderAggregate**. El agregado es el único que puede:
  - Cambiar `order.status` (vía FSM: `validate_transition` + `apply_transition`, sin commit).
  - Crear o modificar la entidad interna `escrow` (asignar `order.escrow_id`, crear `EscrowModel`, actualizar `escrow.status`, hashes, fechas).
- **Identidad:** El agregado se identifica por el id de la orden (`order.id`). Escrow no tiene identidad pública fuera del agregado: se accede como `aggregate.escrow`; las APIs HTTP que hoy exponen escrow por id/order_id siguen siendo lecturas que leen desde persistencia, pero las **escrituras** (crear/actualizar escrow) se realizan únicamente mediante métodos del agregado que actualizan tanto orden como escrow de forma consistente.
- **Consistencia:** Las invariantes cruzadas Order/Escrow se validan en `_validate_cross_invariants()` dentro del agregado; la FSM sigue gobernando las transiciones de `order.status`; el agregado aplica en memoria y el commit lo hace la capa de aplicación (servicios) tras `repository.save(aggregate)`.

---

## C) Cómo se impide la modificación externa de Escrow

- **Repositorio:** Solo expone carga del agregado (`get_for_update(order_id)`) y persistencia (`save(aggregate)`). No hay `EscrowRepository` que permita guardar un escrow suelto; el escrow se persiste siempre como parte del agregado (al guardar el aggregate se añade/actualiza el escrow en la misma sesión).
- **Servicios:**
  - **orders.py:** No asigna `order.escrow_id` ni crea/actualiza `EscrowModel`; delega en el agregado (p. ej. `accept`, `fund_escrow`, `complete`, `refund`, `open_dispute`, `resolve_dispute_release`, `resolve_dispute_refund`). Para “mark locked” se usa el agregado (que en `fund_escrow` puede crear/ligar escrow y transicionar a ONCHAIN_LOCKED).
  - **escrow.py:** `create_escrow` deja de crear/actualizar escrow y orden por su cuenta; obtiene el agregado por `order_id` (vía repositorio, con lock), llama a un método del agregado (p. ej. `fund_escrow(...)`) que crea el escrow interno y actualiza la orden, luego `save(aggregate)` y commit. `update_escrow` deja de hacer `m.status = ...` directo; resuelve el `order_id` desde el escrow_id, carga el agregado con `get_for_update(order_id)`, llama al método de dominio que corresponda (p. ej. `resolve_dispute_release` / `resolve_dispute_refund` o el que sincronice estado escrow con orden), luego save y commit.
- **Rutas:** Siguen llamando a los mismos servicios (create_escrow, update_escrow, etc.); la firma y el contrato HTTP no cambian; la implementación de esos servicios es la que pasa a usar agregado y repositorio.
- **Lecturas:** `get_escrow_by_id`, `get_escrow_by_order_id`, `get_escrow_by_external_id` siguen siendo consultas de solo lectura sobre la tabla escrows; no modifican estado, por tanto no violan “no modificar escrow fuera del agregado”.

---

## D) Cómo se mantiene compatibilidad HTTP

- **Rutas:** No se modifican. Siguen existiendo `POST /orders`, `PUT /orders/{id}/accept`, `PUT /orders/{id}/mark-locked`, `PUT /orders/{id}/complete`, `PUT /orders/{id}/cancel`, `PUT /orders/{id}/dispute`, `GET /orders/...`, y `POST /escrow`, `GET /escrow/...`, `PUT /escrow/{id}` con los mismos parámetros y códigos de respuesta.
- **Esquemas Pydantic:** Order, Escrow, CreateEscrowBody, UpdateEscrowBody, etc. no cambian; los servicios siguen devolviendo los mismos DTOs construidos desde los modelos (por ejemplo `_model_to_order(aggregate.order)`, `_model_to_escrow(aggregate.escrow)` cuando exista).
- **Comportamiento observable:** Misma semántica de negocio: aceptar orden, marcar locked (fund escrow), completar, cancelar, disputar, crear escrow, actualizar escrow; los errores 403/404/409 se mantienen (validaciones de rol, no encontrado, transición inválida o invariante). La única diferencia es interna: las transiciones y los cambios de escrow pasan por el agregado y el repositorio, garantizando consistencia y evitando modificaciones directas fuera del aggregate.

---

## Resumen de flujo por caso de uso

| Caso HTTP | Servicio | Flujo interno (después de refactor) |
|-----------|----------|-------------------------------------|
| PUT /orders/{id}/accept | accept_order | get_for_update(id) → aggregate.accept(buyer) → save + commit (FSM BUYER_ACCEPT) |
| PUT /orders/{id}/mark-locked | mark_order_locked | get_for_update(id) → aggregate.fund_escrow() (o con datos escrow si ya existe) → save + commit (FSM ESCROW_LOCKED) |
| PUT /orders/{id}/complete | complete_order | get_for_update(id) → aggregate.complete() → save + commit (FSM RELEASE_COMPLETE + escrow RELEASED) |
| PUT /orders/{id}/cancel | cancel_order | get_for_update(id) → aggregate.refund(cancelled_by) → save + commit (FSM CANCEL + escrow REFUNDED si existe) |
| PUT /orders/{id}/dispute | dispute_order | get_for_update(id) → aggregate.open_dispute(actor) → save + commit (FSM DISPUTE) |
| POST /escrow | create_escrow | get_for_update(order_id) → aggregate.fund_escrow(...) (crea escrow interno, order.escrow_id, FSM ESCROW_LOCKED) → save + commit |
| PUT /escrow/{id} | update_escrow | resolución order_id por escrow_id → get_for_update(order_id) → aggregate.resolve_dispute_release() o resolve_dispute_refund() según body → save + commit |

Las lecturas (GET orders, GET escrow) no cambian de contrato; pueden seguir usando get_order_by_id, get_escrow_by_id, etc., que no modifican estado.

---

Una vez validado A–D, se implementa: `order_aggregate.py`, `order_repository.py`, refactor de `orders.py` y `escrow.py`, y tests en `test_order_aggregate.py`.
