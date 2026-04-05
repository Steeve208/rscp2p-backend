# FSM de órdenes — Diseño formal (A–D)

Estados y contratos existentes se mantienen; no se modifican esquemas ni rutas HTTP.  
Implementación: `app/domain/order_state_machine.py`.

---

## A) Matriz completa de transición (determinística)

**Conjunto de estados (schema existente):**

- `CREATED`
- `AWAITING_FUNDS`
- `ONCHAIN_LOCKED`
- `COMPLETED`
- `REFUNDED`
- `DISPUTED`
- `PENDING_APPROVAL`

**Estados terminales (sin transiciones salientes):**

- `COMPLETED`
- `REFUNDED`
- `DISPUTED`

*(`PENDING_APPROVAL` no tiene transiciones entrantes en el flujo actual; se deja definido sin eventos que lo alcancen.)*

**Conjunto de eventos:**

- `BUYER_ACCEPT`
- `ESCROW_LOCKED`
- `RELEASE_COMPLETE`
- `CANCEL`
- `DISPUTE`

**Matriz (current_state × event → next_state).** Solo las celdas indicadas están permitidas; cualquier otra combinación es inválida.

| Estado actual     | BUYER_ACCEPT   | ESCROW_LOCKED   | RELEASE_COMPLETE | CANCEL   | DISPUTE  |
|-------------------|----------------|-----------------|------------------|----------|----------|
| CREATED           | AWAITING_FUNDS | —               | —                | REFUNDED | —        |
| AWAITING_FUNDS    | —              | ONCHAIN_LOCKED  | —                | REFUNDED | DISPUTED |
| ONCHAIN_LOCKED    | —              | —               | COMPLETED        | REFUNDED | DISPUTED |
| COMPLETED         | —              | —               | —                | —        | —        |
| REFUNDED          | —              | —               | —                | —        | —        |
| DISPUTED          | —              | —               | —                | —        | —        |
| PENDING_APPROVAL  | —              | —               | —                | —        | —        |

**Regla:** Para cualquier par `(current_state, event)` no presente en esta matriz, la transición se rechaza (409).

---

## B) Tabla evento → roles permitidos

| Evento           | Roles permitidos | Nota breve                                      |
|------------------|-------------------|--------------------------------------------------|
| BUYER_ACCEPT     | BUYER             | Solo quien acepta (no el vendedor).             |
| ESCROW_LOCKED    | BUYER             | Quien deposita en escrow.                        |
| RELEASE_COMPLETE | SELLER, BUYER     | Vendedor libera o comprador confirma recepción.  |
| CANCEL           | SELLER, BUYER     | Quien cancela (seller o buyer según reglas).     |
| DISPUTE          | SELLER, BUYER     | Cualquiera de las dos partes.                    |

El actor se identifica por `actor_role in ("SELLER", "BUYER")` y debe coincidir con la orden (seller_id o buyer_id).

---

## C) Lista de invariantes obligatorios

1. **buyer_id en estados avanzados**  
   Si `status in (AWAITING_FUNDS, ONCHAIN_LOCKED, COMPLETED, REFUNDED, DISPUTED)` entonces `buyer_id` no puede ser `null`.

2. **escrow_id en ONCHAIN_LOCKED y siguientes**  
   Si `status in (ONCHAIN_LOCKED, COMPLETED)` entonces `escrow_id` no puede ser `null`.  
   *(REFUNDED/DISPUTED pueden tener escrow_id; no se exige null.)*

3. **Sin transiciones desde terminales**  
   Si `status in (COMPLETED, REFUNDED, DISPUTED)` no se permite ningún evento (409).

4. **Máximo una disputa abierta por orden**  
   Al aplicar el evento `DISPUTE`, no debe existir ya un registro en `disputes` para esta orden con `status` considerado abierto (p. ej. OPEN, IN_REVIEW). Si no se crea registro de disputa en esta FSM, la condición se cumple por ser DISPUTED terminal (solo se entra una vez).

5. **BUYER_ACCEPT: actor no es el vendedor**  
   Para `BUYER_ACCEPT`, el actor debe ser distinto de `order.seller_id` (quien acepta se convierte en buyer).

---

## D) Coherencia matemática

- **Determinismo:** Cada par `(current_state, event)` permitido tiene exactamente un `next_state`.
- **Terminales:** Ningún evento sale de COMPLETED, REFUNDED ni DISPUTED.
- **Roles:** Cada evento tiene al menos un rol permitido; la validación de rol es previa a la de transición.
- **Invariantes:** Todas son comprobables antes o después de aplicar la transición; si fallan, se rechaza con 409 (o 403 si aplica por rol).
- **Sin ambigüedad:** Cualquier par (estado, evento) no definido en la matriz se considera inválido.

---

Con esto queda cerrado el diseño A–D. El código debe implementar exactamente esta FSM.
