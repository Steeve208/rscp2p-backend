# Auditoría: Arquitectura de pagos del marketplace

**Rol:** Auditor de sistemas financieros  
**Alcance:** Order, Escrow, Ledger, Deposit processor, condiciones de carrera e invariantes.

---

## 1. Resumen ejecutivo

La arquitectura actual **centraliza** las mutaciones en el agregado Order, usa **idempotencia por clave** en depósitos, **transacciones únicas** por operación y **bloqueo de fila** (FOR UPDATE) en el orden. Se identifican **riesgos medios y bajos** y mejoras de defensa en profundidad (validaciones, tests y salvaguardas adicionales).

---

## 2. Riesgos detectados

### R1. Divergencia Order / Escrow por actualización directa en BD

| Atributo   | Valor |
|-----------|--------|
| **Clasificación** | Medio |
| **Componente**    | Persistencia (orders, escrows) |

**Escenario**  
Un script, migración o acceso directo a la BD ejecuta `UPDATE orders SET status = 'RELEASED'` (o `ESCROW_FUNDED`, `CANCELLED`) sin actualizar `escrows.status`, o actualiza solo `escrows` sin tocar `orders`.

**Impacto**  
- Invariantes del dominio se rompen (p. ej. order RELEASED con escrow FUNDED).
- Lógica que asume consistencia (reportes, liberación de fondos, disputas) puede duplicar movimientos o mostrar estados incoherentes.

**Mitigación actual**  
- Toda la lógica de aplicación pasa por el agregado y por `get_for_update` + `repo.save` en la misma transacción; no hay rutas de API que escriban Order/Escrow sin agregado.
- No existe **barrera técnica** en BD (triggers, checks) que impida updates directos.

**Recomendación**  
1. **Fijar en diseño:** Documentar que Order y Escrow **solo** se modifican vía aplicación (agregado + repositorio).  
2. **Defensa en profundidad (opcional):** Añadir un job o endpoint de “consistency check” que lea `(orders.status, escrows.status)` por orden y alerte o marque incoherencias (sin corregir automáticamente sin criterio de negocio).  
3. **Tests:** Añadir test que, ante datos ya divergentes (inyectados en BD), verifique que el agregado o un servicio de consistencia **detecte** la divergencia (p. ej. `_validate_cross_invariants` o un check explícito al cargar).

**Test sugerido**  
- `test_order_escrow_divergence_detected_on_load`: insertar orden con status RELEASED y escrow con status FUNDED; cargar agregado (o ejecutar consistency check); afirmar que se detecta inconsistencia (excepción o flag) y no se permite nueva transición hasta corregir datos.

---

### R2. Ledger y aggregate en transacciones distintas (riesgo teórico)

| Atributo   | Valor |
|-----------|--------|
| **Clasificación** | Bajo (diseño actual correcto) |
| **Componente**    | deposit_processor, orders.complete_order, cancel_order, escrow.update_escrow |

**Escenario**  
Si en algún flujo se hiciera `commit` después de escribir ledger y **antes** de persistir order/escrow, un fallo posterior dejaría ledger con entradas sin estado coherente en Order/Escrow.

**Estado actual**  
- En `deposit_processor`: ledger (`create_balanced_entries`) + `apply_deposit` + `repo.save` + `persist_domain_events` + **un solo** `commit` al final.  
- En `complete_order` / `cancel_order` / `update_escrow`: mutación del agregado + `create_balanced_entries` + `repo.save` + persistencia de eventos + **un solo** `commit`.  
Todo ocurre en **una transacción**; no hay commit intermedio.

**Recomendación**  
- Mantener la regla: “una operación de negocio = una transacción; ledger + order/escrow + eventos en la misma transacción”.  
- Opcional: test que verifique que, si se introduce un `commit()` prematuro en un flujo (p. ej. justo después de ledger), un test de integración falle (p. ej. comprobando que no existan entradas de ledger sin order en estado coherente).

**Test sugerido**  
- Comentado en código o en un test “negative”: “Si se hace commit entre ledger y save(aggregate), el test de invariante ledger/order debe fallar”. No implementar el commit real en producción.

---

### R3. Depósitos duplicados (misma orden, distinto tx_hash)

| Atributo   | Valor |
|-----------|--------|
| **Clasificación** | Bajo (protegido por estado) |
| **Componente**    | deposit_processor, apply_deposit |

**Escenario**  
Llegan dos eventos de depósito para la **misma orden** con **distintos** `tx_hash` (p. ej. usuario envía dos transacciones). La idempotencia es por `(order_id, tx_hash)` o por `idempotency_key`; si el segundo tiene otra clave, podría intentar aplicar.

**Impacto**  
Doble crédito en escrow (ledger y/o estado) si el segundo depósito se aplicara.

**Mitigación actual**  
- `apply_deposit` exige `order.status == "AWAITING_PAYMENT"`. Tras el primer depósito, la orden pasa a `ESCROW_FUNDED`.  
- El segundo evento, con otro `tx_hash`, intentaría `apply_deposit` y recibiría 409 (“Deposit only allowed in AWAITING_PAYMENT”).  
- No se escriben ledger ni cambios de escrow en el segundo intento.

**Recomendación**  
- Dejar el comportamiento actual.  
- Opcional: en `deposit_processor`, si `order.status != "AWAITING_PAYMENT"` y el evento es “deposit”, marcar el evento como REJECTED con motivo “Order not in AWAITING_PAYMENT” en lugar de solo hacer commit del evento en estado PENDING y devolver 409, para trazabilidad en `deposit_events`.

**Test sugerido**  
- `test_second_deposit_same_order_different_tx_rejected`: orden ya ESCROW_FUNDED por tx1; procesar segundo evento (order_id mismo, tx_hash distinto); afirmar 409 o resultado rechazado y que el balance de escrow en ledger no cambie (sigue siendo el monto del primer depósito).

---

### R4. Condición de carrera: idempotencia y “cached result” antes de commit

| Atributo   | Valor |
|-----------|--------|
| **Clasificación** | Bajo |
| **Componente**    | deposit_processor |

**Escenario**  
Worker A inserta `deposit_events` (claim) y entra en el flujo (lock order, ledger, apply_deposit, …). Worker B intenta el mismo `idempotency_key`, recibe IntegrityError, hace rollback y llama `_get_cached_result(db, key)`. Si A **aún no ha hecho commit**, B podría no ver la fila de A y recibir `None` de `_get_cached_result`, devolviendo entonces el resultado “fallback” (ESCROW_FUNDED, already_processed=True) sin haber comprobado en BD.

**Impacto**  
- B devuelve “éxito en caché” aunque A no haya terminado.  
- Si A falla después (p. ej. 404 o 409), el estado final podría ser “no aplicado”, pero B ya habría dicho “already_processed”. Idempotencia de respuesta sigue siendo aceptable para el cliente (reintento con misma clave sigue devolviendo lo mismo), pero el estado real podría ser “aún no procesado” hasta que otro reintento con la misma clave vuelva a intentar y entonces sí aplicar o ver el resultado real.

**Mitigación actual**  
- Claim-first: solo un worker puede tener la fila en `deposit_events`.  
- Los que pierden el claim leen después; en el peor caso leen antes del commit de A y devuelven el fallback; un nuevo request con la misma clave podría ver ya la fila PROCESSED o volver a intentar el claim (y perder y leer cached).

**Recomendación**  
- Opcional: tras IntegrityError, reintentar una vez `_get_cached_result` después de un breve sleep (p. ej. 50–100 ms) para dar tiempo al commit del ganador, y solo si sigue siendo None devolver el fallback.  
- Documentar que “already_processed” puede ser “optimistic” hasta que el ganador haga commit.

**Test sugerido**  
- Test de estrés ya existente (20 workers, mismo order y misma clave) verifica que solo un depósito se aplica y que las invariantes de ledger/order se cumplen. Opcional: test que simule dos workers (uno que “gana” y tarda en commit) y el otro que devuelve cached; afirmar que tras commit del ganador, un tercer request con la misma clave ve resultado coherente (PROCESSED, order ESCROW_FUNDED).

---

### R5. Ledger: entradas desbalanceadas por bug en llamadas

| Atributo   | Valor |
|-----------|--------|
| **Clasificación** | Bajo |
| **Componente**    | ledger_service.create_balanced_entries, callers |

**Escenario**  
Un caller construye mal la lista de entradas (p. ej. solo crédito a escrow sin débito, o suma distinta de cero por moneda) y llama `create_balanced_entries`.

**Impacto**  
- `create_balanced_entries` lanza `LedgerError` y la transacción hace rollback; no se persisten entradas desbalanceadas.  
- Si en el futuro alguien añadiera un path que escriba en `ledger_entries` sin pasar por `create_balanced_entries`, podría romper la invariante “suma = 0 por moneda”.

**Mitigación actual**  
- Validación en `create_balanced_entries`: suma por moneda debe ser 0; si no, `LedgerError`.  
- No hay otros puntos que inserten en `ledger_entries` salvo vía este servicio en los flujos revisados.

**Recomendación**  
- Mantener una única puerta de escritura en ledger: `create_balanced_entries` (y/o helpers que siempre construyan pares balanceados).  
- Rechazar en code review cualquier INSERT directo a `ledger_entries` fuera del servicio de ledger.

**Test sugerido**  
- Ya existe test que verifica que entradas desbalanceadas lanzan. Añadir test que verifique que, para un order_id, la suma de `amount` por (order_id, currency) en `ledger_entries` es 0 (invariante global por orden/moneda) tras cualquier flujo completo (deposit, release, refund).

---

### R6. complete_order / cancel_order: orden de operaciones y fallo de ledger

| Atributo   | Valor |
|-----------|--------|
| **Clasificación** | Bajo |
| **Componente**    | orders.complete_order, cancel_order |

**Escenario**  
Se llama `agg.complete()` (o `agg.refund()`), que muta order y escrow en memoria; luego `create_balanced_entries` falla (p. ej. LedgerError por lista mal formada). La transacción hace rollback y no se hace commit.

**Impacto**  
Ninguno persistido: ni order/escrow ni ledger. Comportamiento correcto.

**Recomendación**  
Sin cambio. Opcional: documentar en comentario que el orden “aggregate mutation in memory → ledger → repo.save → commit” garantiza todo-o-nada.

**Test sugerido**  
- Test que mockee o inyecte fallo en `create_balanced_entries` tras `complete()` y verifique que la orden **no** cambia de estado en BD (rollback).

---

### R7. Escrow RELEASED/REFUNDED sin entradas de ledger (update_escrow)

| Atributo   | Valor |
|-----------|--------|
| **Clasificación** | Bajo |
| **Componente**    | escrow.update_escrow |

**Escenario**  
`update_escrow` con status RELEASED o REFUNDED llama a `resolve_dispute_release` / `resolve_dispute_refund` y luego `create_balanced_entries` (release o refund). Si alguien en el futuro añade un path que solo actualice escrow/order a RELEASED o REFUNDED sin escribir ledger, el balance derivado del ledger no coincidiría con el estado del contrato.

**Estado actual**  
- En los flujos revisados, release y refund siempre van acompañados de `create_balanced_entries` en la misma transacción.

**Recomendación**  
- Mantener la regla: “cualquier transición a RELEASED o REFUNDED que represente movimiento de fondos debe generar entradas de ledger en la misma transacción”.  
- Opcional: en el agregado o en un servicio, antes de persistir, comprobar que para la orden existe al menos una entrada de tipo RELEASE o REFUND cuando el estado final es RELEASED o CANCELLED (refund); sería una verificación de consistencia estricta.

**Test sugerido**  
- Tras `complete_order` o `resolve_dispute_release`, afirmar que existe al menos una fila en `ledger_entries` para ese order_id con type RELEASE y que el balance de escrow para esa moneda disminuyó en el monto esperado.

---

## 3. Clasificación de riesgos

| Id  | Riesgo                                      | Clasificación | Estado actual              |
|-----|---------------------------------------------|---------------|----------------------------|
| R1  | Divergencia Order/Escrow por updates directos | Medio         | Mitigado por diseño        |
| R2  | Ledger y aggregate en transacciones distintas | Bajo          | No aplicable (una transacción) |
| R3  | Doble depósito misma orden distinto tx      | Bajo          | Protegido por estado       |
| R4  | Cached result antes de commit del ganador    | Bajo          | Aceptable; documentar      |
| R5  | Ledger desbalanceado por bug                 | Bajo          | Validación en servicio     |
| R6  | Fallo de ledger tras complete/refund         | Bajo          | Rollback correcto          |
| R7  | Release/refund sin ledger                     | Bajo          | No ocurre en flujos actuales |

---

## 4. Resumen de correcciones recomendadas

1. **R1:** Añadir comprobación de consistencia Order/Escrow (job o endpoint) y test que detecte divergencia al cargar.  
2. **R3 (opcional):** Rechazar explícitamente en `deposit_processor` cuando `order.status != "AWAITING_PAYMENT"` y guardar REJECTED en `deposit_events` con motivo claro.  
3. **R4 (opcional):** Reintento con pequeño delay al leer cached result tras IntegrityError, y documentar semántica “optimistic” de already_processed.  
4. **R5/R7:** Mantener una sola puerta de escritura en ledger y tests que afirmen invariantes de suma por orden/moneda y existencia de entradas RELEASE/REFUND cuando corresponde.

---

## 5. Tests a añadir (checklist)

- [ ] **test_order_escrow_divergence_detected_on_load**: datos divergentes (p. ej. order RELEASED, escrow FUNDED); cargar agregado o ejecutar consistency check; afirmar detección de incoherencia.
- [ ] **test_second_deposit_same_order_different_tx_rejected**: orden ya ESCROW_FUNDED; segundo evento mismo order_id, otro tx_hash; afirmar 409/rechazo y que balance de escrow no cambie.
- [ ] **test_ledger_balance_invariant_per_order_currency**: tras deposit + release (o refund), afirmar que para ese order_id y currency la suma de `ledger_entries.amount` es 0 (o el valor esperado según flujo).
- [ ] **test_release_creates_ledger_entries**: tras complete_order, afirmar que existen entradas de tipo RELEASE para el order_id y que el balance de escrow para esa moneda es 0 (o el esperado).
- [ ] **test_rollback_on_ledger_failure_after_complete**: simular fallo en `create_balanced_entries` después de `agg.complete()`; afirmar que order no queda en RELEASED en BD (transacción en rollback).

---

## 6. Conclusión

La arquitectura es sólida: agregado único, idempotencia por evento de depósito, transacción única por operación y bloqueo de fila evitan la mayoría de condiciones de carrera y divergencias. Los riesgos restantes son mayormente de **defensa en profundidad** (detección de divergencia, tests de invariantes y documentación). Priorizar R1 (detección de divergencia + test) y el resto según política de riesgo.
