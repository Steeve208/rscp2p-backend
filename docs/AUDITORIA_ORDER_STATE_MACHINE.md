# Auditoría del módulo `order_state_machine.py`

**Alcance:** Exclusivamente `app/domain/order_state_machine.py`.  
**Criterios:** Transiciones implícitas, fugas desde terminales, no-determinismo, modificaciones directas de `status`, condiciones de carrera.  
**Severidades:** Crítica | Alta | Media | Baja | Informativa.

---

## 1. Transiciones implícitas no declaradas

### 1.1 Estado declarado sin participación en la matriz — **Media**

- **Hecho:** `ORDER_STATES` incluye `"PENDING_APPROVAL"` (línea 28), pero no existe ninguna entrada en `TRANSITION_MATRIX` que tenga `PENDING_APPROVAL` como estado origen ni como estado destino.
- **Riesgo:** Un orden con `status = "PENDING_APPROVAL"` (p. ej. por migración o bug) sería considerado por `validate_transition` como estado no terminal; para cualquier evento la matriz devuelve `None`, por lo que toda transición falla con 409. El estado es inalcanzable por la FSM y sin salida definida.
- **Recomendación:** Decidir si `PENDING_APPROVAL` es estado legítimo del flujo. Si sí: añadir entradas en `TRANSITION_MATRIX` (y opcionalmente en `TERMINAL_STATES` si aplica). Si no: quitarlo de `ORDER_STATES` para evitar estados “zombie” y documentar en el diseño que no se usa.

### 1.2 Consistencia con esquema

- **Hecho:** El esquema Pydantic (`OrderStatus`) incluye los mismos valores que `ORDER_STATES` (incl. `PENDING_APPROVAL`). No se encontró uso de `CANCELLED`; el flujo de cancelación usa `REFUNDED` de forma coherente.

---

## 2. Estados terminales que puedan salir

### 2.1 Bloqueo formal de salida — **Cumplido**

- **Hecho:** En `validate_transition` (líneas 79–80) si `current_state in TERMINAL_STATES` se devuelve `None`, por lo que no hay transición definida desde terminales.
- **Hecho:** En `transition_order` (líneas 172–176) si `order.status in TERMINAL_STATES` se lanza `HTTPException 409` antes de consultar la matriz.
- **Conclusión:** No existe transición permitida que permita salir de `COMPLETED`, `REFUNDED` o `DISPUTED`. No se detectan fugas desde estados terminales.

---

## 3. Eventos que permitan múltiples resultados

### 3.1 Determinismo de la matriz — **Cumplido**

- **Hecho:** `TRANSITION_MATRIX` es un `dict[tuple[str, OrderEvent], str]`. Cada par `(current_state, event)` tiene como máximo una imagen (un único `next_state`).
- **Hecho:** No hay ramas en el código que elijan un `next_state` distinto según otro criterio; el siguiente estado se obtiene solo de la matriz.
- **Conclusión:** No hay eventos que permitan múltiples resultados; la FSM es determinística en (estado, evento) → siguiente estado.

---

## 4. Violaciones de determinismo

### 4.1 Matriz y aplicación — **Cumplido**

- La única asignación a `order.status` dentro del módulo está en `apply_transition` (línea 137), y el valor viene de `validate_transition(current, event)`, es decir, de la matriz.
- Los timestamps y campos derivados (`accepted_at`, `completed_at`, etc.) dependen solo del evento y de `datetime.now(timezone.utc)`, sin ramas que alteren el estado resultante de la transición.
- **Conclusión:** No se detectan violaciones de determinismo en la lógica de transición.

---

## 5. Modificaciones directas de `status` fuera de `apply_transition`

### 5.1 Dentro del módulo — **Cumplido**

- **Hecho:** En `order_state_machine.py` el único lugar donde se asigna `order.status` es en `apply_transition` (línea 137). `transition_order` no modifica `order.status` directamente.
- **Conclusión:** Cumplimiento correcto dentro del módulo.

### 5.2 Uso en el resto de la aplicación — **Informativa**

- **Hecho:** En `app/services/orders.py`, `create_order` asigna `status="CREATED"` al crear una nueva orden (estado inicial, no transición). Las transiciones (accept, mark_locked, complete, cancel, dispute) se realizan mediante `OrderAggregate` y `apply_transition` desde `app/domain/order_aggregate.py`, que a su vez usa la FSM. No se asigna `OrderModel.status` directamente en los servicios de transición.
- **Hecho:** En `escrow.py`, las actualizaciones de estado de escrow que afectan a la orden (resolve_release, resolve_refund, record_escrow_locked) pasan por el agregado; el `m.status` en lecturas/mapeo corresponde a `EscrowModel`.
- **Conclusión:** No hay modificaciones directas de `status` de órdenes fuera de la FSM más allá del estado inicial en creación. Correcto.

---

## 6. Posibles condiciones de carrera

### 6.1 Ausencia de bloqueo pesimista — **RESUELTO**

- **Estado actual:** Los servicios de órdenes usan `OrderRepository.get_for_update(order_id)` (SELECT FOR UPDATE). La carga con bloqueo, el agregado y el commit ocurren en la misma sesión; no hay ventana de lost update. La FSM no hace la carga; el llamador (servicio) obtiene el agregado bajo lock y luego commitea tras `repo.save(agg)`. **Hecho anterior:** `transition_order` recibe el objeto `order` ya cargado por el llamador. En `app/services/orders.py`, la orden se obtiene con `db.get(OrderModel, order_id)` sin `with_for_update()` ni equivalente. No existe uso de `SELECT ... FOR UPDATE` en el flujo de órdenes.
- **Riesgo:** Dos peticiones concurrentes pueden cargar la misma orden en el mismo estado, pasar ambas las validaciones (rol, terminal, matriz, invariantes), aplicar la misma o distinta transición en memoria y hacer `commit()` de forma secuencial. El último commit sobrescribe al anterior (posible “lost update”) o se aplican dos transiciones que deberían ser mutuamente excluyentes (p. ej. una completa y otra cancela).
- **Ejemplo:** Request A: GET order (ONCHAIN_LOCKED) → RELEASE_COMPLETE → commit (COMPLETED). Request B: GET order (ONCHAIN_LOCKED) → CANCEL → commit (REFUNDED). Según el orden de commit, el estado final puede ser REFUNDED habiendo ya completado, o COMPLETED habiendo ya cancelado.
- **Recomendación:** Bloquear la fila al leer la orden para transicionar, por ejemplo cargando con `db.get(OrderModel, order_id, with_for_update=True)` o equivalente según el dialecto (e.g. `with_for_update()` en la query), de forma que la transición y el commit se ejecuten bajo lock. Mantener la transacción corta para reducir contención.

### 6.2 Orden validación → aplicación → commit — **Informativa**

- **Hecho:** El flujo es: validar rol, terminal, transición e invariantes; luego `apply_transition(order, ...)`; luego `db.commit()` y `db.refresh(order)`. No se vuelve a leer la orden desde la BD entre la validación y el commit.
- **Conclusión:** La ventana de carrera está entre la carga inicial de la orden (en el servicio) y el commit. El cierre de esa ventana requiere bloqueo en la carga, no cambios adicionales dentro de `transition_order` más allá de documentar que el llamador debe cargar la orden bajo lock.

---

## 7. Otras observaciones

### 7.1 Docstring de invariantes — **Baja**

- **Hecho:** En `_check_invariants` (líneas 91–92) el comentario dice “buyer_id no null cuando next_state >= AWAITING_FUNDS”. La implementación usa `STATES_REQUIRING_BUYER`, que solo incluye `AWAITING_FUNDS`, `ONCHAIN_LOCKED`, `COMPLETED` (no REFUNDED ni DISPUTED).
- **Recomendación:** Ajustar el docstring para que diga explícitamente que solo se exige `buyer_id` en esos tres estados (p. ej. “buyer_id no null cuando next_state in (AWAITING_FUNDS, ONCHAIN_LOCKED, COMPLETED)”).

### 7.2 Doble validación de transición — **Informativa**

- **Hecho:** `transition_order` obtiene `next_state = validate_transition(...)` (línea 178) y luego `apply_transition` vuelve a llamar a `validate_transition` (línea 132). Redundante pero coherente; no afecta determinismo ni seguridad.

---

## 8. Resumen por severidad

| Severidad  | Cantidad | Ítems |
|------------|----------|--------|
| **Crítica** | 0 | 6.1 resuelto: servicios usan `OrderRepository.get_for_update()` (SELECT FOR UPDATE). |
| **Alta**    | 0 | — |
| **Media**   | 1 | Estado `PENDING_APPROVAL` declarado sin transiciones (1.1). |
| **Baja**    | 1 | Docstring de invariantes desalineado con la implementación (7.1). |
| **Informativa** | 4 | Consistencia esquema/FSM (1.2), determinismo matriz (3.1), modificaciones de status fuera del módulo (5.2), orden validación–commit (6.2), doble validación (7.2). |

---

## 9. Conclusión

El módulo es **determinístico**, **no permite salir de estados terminales** y **concentra todas las transiciones de estado en `apply_transition`** sin modificaciones directas de `order.status` fuera de ella. El hallazgo crítico de condiciones de carrera (6.1) está **resuelto** en la capa de aplicación: los servicios de órdenes usan `OrderRepository.get_for_update(order_id)` para cargar con `SELECT ... FOR UPDATE` antes de invocar al agregado (que utiliza la FSM). El estado `PENDING_APPROVAL` sigue pendiente de decisión (transiciones o eliminación de `ORDER_STATES`).
