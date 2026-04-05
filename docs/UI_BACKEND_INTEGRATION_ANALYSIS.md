# Análisis: Interfaz P2P Marketplace → Requisitos Backend

**Objetivo:** Reverse-engineer del frontend para determinar qué debe proveer el backend. Sin generar código frontend; solo requisitos de integración.

---

## 1. Inventario de componentes UI

| Componente | Ubicación / Uso | Descripción breve |
|------------|------------------|-------------------|
| **Tabla de ofertas (marketplace list)** | `/marketplace` – `OffersTable` | Lista de órdenes con status CREATED como ofertas (compra/venta). Filtros: búsqueda, crypto, tipo BUY/SELL, método de pago, confianza, monto. |
| **Filtros de búsqueda** | `SearchFilters`, `MarketplaceMobileMenu` | searchTerm, selectedCrypto, tradeType, paymentMethod, trustLevel, amount. |
| **Lista “Mis órdenes”** | `/marketplace/orders` | Tabla de órdenes del usuario (buyer o seller). Filtros: tipo (all/buy/sell), estado (all/in_progress/completed/cancelled), búsqueda por ID. |
| **Badges de estado** | Orders page, Buy page | Agrupación: “En progreso” (CREATED, AWAITING_FUNDS, ONCHAIN_LOCKED, PENDING_APPROVAL), “Completada”, “Cancelada”, “En disputa”. |
| **Botones de acción (lista órdenes)** | Orders page | “Al pago” (link a transaction?orderId=), “VER” (link a buy/[id]). Solo “Al pago” si inProgress. |
| **Página detalle oferta / compra** | `/marketplace/buy/[id]` | Detalle de una orden: vendedor, nivel/reputación, cantidad, precio, método de pago, términos, resumen, instrucciones. Acciones: CONFIRMAR (accept), CANCELAR, ABRIR DISPUTA, CONTINUAR AL PAGO. |
| **Panel resumen (buy page)** | Buy page | Cantidad, precio unitario, método de pago, total a pagar. |
| **Detalles de escrow (buy page)** | Buy page | Texto fijo: fondos en escrow on-chain, wallet to wallet, liberación tras confirmación. |
| **Modal disputa** | Buy page, Transaction page | Textarea “motivo”, botones Cancelar / Enviar disputa. |
| **Página transacción en progreso** | `/marketplace/transaction?orderId=` | Indicador de pasos (1–4): oferta aceptada, fondos en escrow, pago/confirmación, completado. Info de pago (desde/hacia, monto, red). Acciones: comprador “Pagar con USDT/USDC” o “YA PAGUE”; vendedor “Confirmar recepción y liberar”; Cancelar; Abrir disputa. |
| **Paso a paso (stepper)** | Transaction page | 1) Oferta aceptada, 2) Fondos en escrow, 3) Pago/confirmación, 4) Completado. Derivado de order.buyerId, escrow status, order.status. |
| **Información de pago** | Transaction page | Desde/hacia wallet, monto, red (blockchain). |
| **Backend status** | `/marketplace` | Componente que indica estado del backend (salud). |
| **Página disputas** | `/marketplace/disputes` | Contenido estático (flujo de disputa, links a órdenes y FAQ). No lista disputas desde API. |
| **Crear oferta** | `/marketplace/offers/new` | Formulario para crear orden (vendedor). |
| **Wallet connect** | Varias páginas | Conexión de wallet; no es dato de backend. |

---

## 2. Requisitos de datos por componente

### 2.1 Tabla de ofertas (marketplace list)

| Dato | Origen backend | Notas |
|------|----------------|-------|
| Lista de órdenes | GET /orders?page&limit (status=CREATED para “ofertas”) | Solo CREATED para listar ofertas. |
| id | order.id | |
| sellerId, seller.wallet_address, seller.reputation_score | order.sellerId, order.seller | Para nivel/etiqueta vendedor. |
| cryptoCurrency, cryptoAmount, fiatCurrency, fiatAmount, pricePerUnit | order.* | |
| paymentMethod | order.paymentMethod | |
| status | order.status | Debe ser CREATED para aparecer como oferta. |
| createdAt | order.createdAt | Opcional para ordenación. |

Filtros actuales en UI (searchTerm, selectedCrypto, tradeType, paymentMethod, trustLevel, amount) se aplican en cliente; el backend ya soporta status, sellerId, buyerId, cryptoCurrency, fiatCurrency. Para trustLevel/amount haría falta extensión (reputación por seller, rango de monto) o filtrar en frontend.

### 2.2 Mis órdenes

| Dato | Origen backend | Notas |
|------|----------------|-------|
| Lista órdenes del usuario | GET /orders/me?role=seller|buyer|both&page&limit | Con token. |
| Por orden: id, type (Venta/Compra), amount (crypto), counterpart (wallet), total (fiat), status (etiqueta), inProgress | Order + derivados | type según si soy seller o buyer; counterpart la contraparte; status agrupado (ver 2.5). |

### 2.3 Detalle oferta (buy page)

| Dato | Origen backend | Notas |
|------|----------------|-------|
| order_id | route param | |
| Order completo | GET /orders/{id} | Incluir seller, buyer cuando existan. |
| seller.wallet_address, seller.reputation_score | order.seller | Para nombre y “level”. |
| cryptoAmount, cryptoCurrency, fiatAmount, fiatCurrency, pricePerUnit, paymentMethod, terms | order.* | |
| status, buyerId | order.status, order.buyerId | Para canAccept, canCancel, canDispute, goToTransaction. |
| blockchain | No existe en Order actual | Opcional; frontend usa `(order as any).blockchain ?? 'Ethereum'`. |

### 2.4 Página transacción (?orderId=)

| Dato | Origen backend | Notas |
|------|----------------|-------|
| order | GET /orders/{id} | |
| escrow | GET /escrow/order/{orderId} | Para isLocked (status LOCKED / FUNDED). |
| order.status, order.buyerId, order.seller, order.buyer | order.* | Para pasos y acciones. |
| order.cryptoAmount, order.fiatAmount, order.fiatCurrency, order.createdAt | order.* | Resumen y fechas. |
| Escrow status | escrow.status | Frontend espera LOCKED; backend tiene FUNDED. Ver Gaps. |

### 2.5 Badges de estado (mapeo UI ↔ backend)

UI agrupa así:

- **En progreso:** CREATED, AWAITING_FUNDS, ONCHAIN_LOCKED, PENDING_APPROVAL  
- **Completada:** COMPLETED  
- **Cancelada:** REFUNDED  
- **En disputa:** DISPUTED  

Backend devuelve: CREATED, AWAITING_PAYMENT, ESCROW_FUNDED, RELEASED, CANCELLED, DISPUTED.

Por tanto hace falta un **mapeo** en backend o en frontend:

| Backend status | Equivalente lógico UI |
|----------------|------------------------|
| CREATED | En progreso (CREATED) |
| AWAITING_PAYMENT | En progreso (AWAITING_FUNDS) |
| ESCROW_FUNDED | En progreso (ONCHAIN_LOCKED / PENDING_APPROVAL) |
| RELEASED | Completada (COMPLETED) |
| CANCELLED | Cancelada (REFUNDED) |
| DISPUTED | En disputa (DISPUTED) |

Recomendación: backend puede exponer un campo opcional `displayStatus` o el frontend mapear en una capa de transformación.

### 2.6 Escrow (transaction page)

| Dato | Origen backend | Notas |
|------|----------------|-------|
| Escrow por orden | GET /escrow/order/{orderId} | 404 si no hay escrow; frontend asume null. |
| status | escrow.status | Backend: PENDING, FUNDED, RELEASED, REFUNDED. Frontend: PENDING, LOCKED, RELEASED, REFUNDED. “Locked” = fondos bloqueados = FUNDED. |
| createTransactionHash, contractAddress, cryptoAmount, cryptoCurrency | escrow.* | Para mostrar datos on-chain. |

---

## 3. Acciones UI → Comandos backend

| Acción UI | Llamada frontend | Comando backend requerido |
|-----------|------------------|----------------------------|
| Crear oferta (vendedor) | createOrder(data, token) | POST /orders (body: cryptoCurrency, cryptoAmount, fiatCurrency, fiatAmount, pricePerUnit, paymentMethod, terms, expiresAt, …). |
| Confirmar compra (aceptar oferta) | acceptOrder(offerId, token) | PUT /orders/{id}/accept (opcional body: paymentMethod). |
| Ir al pago / Continuar al pago | Navegación a /marketplace/transaction?orderId= | Ninguna; solo lectura order + escrow. |
| Marcar fondos bloqueados (comprador) | markOrderLocked(orderId, token) | PUT /orders/{id}/mark-locked. |
| Completar (liberar al comprador) | completeOrder(orderId, token) | PUT /orders/{id}/complete. |
| Cancelar orden | cancelOrder(orderId, token) | PUT /orders/{id}/cancel. |
| Abrir disputa | createDispute({ orderId, reason }, token) | **Gap:** frontend hace POST /disputes con { orderId, reason }; backend tiene PUT /orders/{id}/dispute sin body reason. Ver sección 6. |
| Reintentar / Actualizar estado | refetch order / escrow | GET /orders/{id}, GET /escrow/order/{orderId}. |

---

## 4. Endpoints API requeridos (por página)

### 4.1 Marketplace list

- **GET /orders** – query: page, limit; opcional: status, sellerId, buyerId, cryptoCurrency, fiatCurrency.  
  Para “ofertas” el frontend filtra status === 'CREATED'; puede enviar status=CREATED.

### 4.2 Detalle oferta (buy)

- **GET /orders/{id}** – Order con seller y buyer (cuando existan).  
- **PUT /orders/{id}/accept** – Aceptar (comprador).  
- **PUT /orders/{id}/cancel** – Cancelar (vendedor en CREATED).  
- **POST /disputes** (o **PUT /orders/{id}/dispute** con body) – Abrir disputa con motivo. Actualmente solo PUT sin reason.

### 4.3 Mis órdenes

- **GET /orders/me** – query: role (seller|buyer|both), status, page, limit. Auth requerida.

### 4.4 Transacción en progreso

- **GET /orders/{id}** – Order completo.  
- **GET /escrow/order/{orderId}** – Escrow de la orden (puede 404).  
- **PUT /orders/{id}/mark-locked** – Comprador marca “fondos bloqueados”.  
- **PUT /orders/{id}/complete** – Seller o buyer completa (liberar).  
- **PUT /orders/{id}/cancel** – Cancelar.  
- **POST /disputes** (o PUT /orders/{id}/dispute con reason) – Abrir disputa.

### 4.5 Crear oferta

- **POST /orders** – Crear orden (vendedor). Body según CreateOrderBody.

### 4.6 Disputas (página estática)

- No hay lista de disputas desde API en la UI actual. Si se añade “Mis disputas”, harían falta **GET /disputes** (o GET /orders/me con status=DISPUTED y relación a disputas).

### 4.7 Auth y usuario

- **POST /auth/challenge**, **POST /auth/verify**, **GET /auth/me**, **POST /auth/refresh**, **POST /auth/logout**.  
- **GET /users/wallet/{address}**, **GET /users/stats/{address}** – Para perfil y reputación.

---

## 5. Necesidades de tiempo real

| Componente | Necesidad | Recomendación |
|------------|-----------|----------------|
| Estado de orden (transaction, buy) | Actualización tras depósito/liberación/cancelación | **Polling:** refetch cada N s o tras acción. WebSocket opcional para evitar polling. |
| Lista “Mis órdenes” | Ver nuevos estados sin recargar | Polling o invalidateQueries tras mutaciones; WebSocket opcional. |
| Confirmación de depósito on-chain | Backend recibe webhook y actualiza orden/escrow | Backend ya tiene deposit processor; UI solo necesita refetch o notificación. |
| Chat / mensajes | No hay chat en la UI actual | N/A. |
| Countdown (expiración oferta) | order.expiresAt existe en schema | Frontend puede contar en cliente; no requiere stream. |

Conclusión: no hay requisito estricto de WebSocket; **polling + invalidateQueries** cubren la UI actual. Para mejor UX, un **event stream o WebSocket** (p. ej. “order updated”) permitiría actualizar estado sin polling.

---

## 6. Gaps entre UI y backend

### 6.1 Disputas: método y cuerpo

- **UI:** `createDispute({ orderId, reason }, token)` → **POST /disputes** con body `{ orderId, reason }`.  
- **Backend:** Solo **PUT /orders/{id}/dispute** (sin body reason).  
- **Impacto:** El motivo de la disputa no se persiste; la UI lo envía pero el backend no lo usa.  
- **Recomendación:**  
  - Opción A: Añadir **POST /disputes** con body `{ orderId, reason }` que cree la fila en `disputes` y llame a la lógica de apertura de disputa del agregado (open_dispute).  
  - Opción B: Extender **PUT /orders/{id}/dispute** con body `{ reason?: string }` y guardar reason en `disputes`.

### 6.2 Estados de orden (nombres)

- **UI:** CREATED, AWAITING_FUNDS, ONCHAIN_LOCKED, PENDING_APPROVAL, COMPLETED, REFUNDED, DISPUTED.  
- **Backend:** CREATED, AWAITING_PAYMENT, ESCROW_FUNDED, RELEASED, CANCELLED, DISPUTED.  
- **Impacto:** Lógica en frontend (STATUS_GROUP, goToTransaction, isDone, etc.) usa los nombres de la UI. Si el backend solo devuelve los suyos, el frontend debe mapear.  
- **Recomendación:**  
  - Backend sigue devolviendo status interno.  
  - Frontend mantiene un mapeo único (backendStatus → displayStatus / etiquetas).  
  - Opcional: backend añade en la respuesta `displayStatus` con uno de los valores que espera la UI (o un enum “frontend”).

### 6.3 Estado de escrow: LOCKED vs FUNDED

- **UI:** Escrow “locked” = `escrow.status === 'LOCKED'` o order en ONCHAIN_LOCKED / AWAITING_FUNDS.  
- **Backend:** escrow.status = PENDING | **FUNDED** | RELEASED | REFUNDED.  
- **Impacto:** Código como `escrowData?.status === 'LOCKED'` nunca es true si el backend devuelve FUNDED.  
- **Recomendación:**  
  - En API de escrow, devolver `status: "LOCKED"` cuando el modelo interno sea FUNDED (alias en el schema de respuesta), **o**  
  - Frontend trata `FUNDED` como “locked” (cambio en frontend).

### 6.4 Cancel: body cancelledBy

- **UI:** cancelOrder(id, token) – no envía body.  
- **Backend:** cancel_order(db, order_id, user, cancelled_by) con cancelled_by derivado de order (seller/buyer). La ruta ya deduce cancelled_by.  
- **Conclusión:** Sin gap; no hace falta body.

### 6.5 Campo blockchain / red

- **UI:** Muestra “Red” (blockchain) en buy y transaction; usa `(order as any).blockchain ?? 'Ethereum'`.  
- **Backend:** Order no tiene campo blockchain; CreateOrderBody tiene blockchain opcional.  
- **Recomendación:** Si el backend persiste/expone `blockchain` (o chainId), incluirlo en GET /orders/{id} y en Order schema para no depender de any.

### 6.6 Bug frontend: `isMockData` no definido (buy page)

- En `app/marketplace/buy/[id]/page.tsx` se usa `!isMockData` en la condición `canAccept`. La variable no está definida en el componente.
- **Impacto:** En runtime `isMockData` es `undefined`, por lo que `!isMockData` es true; la lógica de “puede aceptar” no se rompe, pero es un error de código. No es requisito de backend; solo anotación para corrección en frontend.

### 6.7 Listado de disputas

- **UI:** Página “Disputas” es estática; no hay GET /disputes en uso.  
- **Backend:** No hay router /disputes; existe modelo DisputeModel.  
- **Recomendación:** Si más adelante se lista “Mis disputas”, añadir **GET /disputes** (o GET /orders/me con expand=disputes) y documentar schema Dispute.

---

## 7. Estructura de API recomendada

### 7.1 Órdenes (existentes; ajustes menores)

- GET /orders – listado; filtros: status, sellerId, buyerId, cryptoCurrency, fiatCurrency, page, limit.  
- GET /orders/me – idem con role; auth.  
- GET /orders/{id} – detalle; incluir seller/buyer embebidos.  
- GET /orders/{id}/status – { id, status, escrowId, updatedAt }.  
- POST /orders – crear (auth).  
- PUT /orders/{id}/accept – aceptar (auth; body opcional paymentMethod).  
- PUT /orders/{id}/mark-locked – marcar locked (auth; buyer).  
- PUT /orders/{id}/complete – completar (auth).  
- PUT /orders/{id}/cancel – cancelar (auth).  
- PUT /orders/{id}/dispute – abrir disputa (auth). **Añadir body:** `{ "reason": "string" }` y persistir en tabla disputes.

### 7.2 Disputas (nuevo o unificado)

- **Opción recomendada:** Mantener **PUT /orders/{id}/dispute** con body `{ reason: string }` y en backend crear/actualizar fila en `disputes` con reason.  
- **Alternativa:** **POST /disputes** con body `{ orderId, reason }` que internamente llame a la misma lógica que put_dispute_order (agregado + persistir dispute con reason).

### 7.3 Escrow (existentes)

- GET /escrow/order/{orderId} – escrow por orden (404 si no hay).  
- Respuesta: incluir status; si se desea alinear con UI, exponer "LOCKED" cuando el estado interno sea FUNDED (o documentar que FUNDED = locked).

### 7.4 Usuarios y auth

- Sin cambios; GET /users/wallet/{address}, GET /users/stats/{address}, auth challenge/verify/me.

### 7.5 Eventos en tiempo real (opcional)

- **Orden actualizada:** evento o WebSocket “order:{id}:updated” con { status, escrowId, updatedAt } para que la UI actualice sin polling.  
- **Depósito confirmado:** el backend ya procesa vía deposit processor; la UI puede refetch order/escrow o suscribirse al evento anterior.

---

## 8. Resumen de acciones recomendadas en backend

1. **Disputa con motivo:** Añadir body `{ reason?: string }` a **PUT /orders/{id}/dispute** y persistir reason en tabla disputes (o implementar POST /disputes con orderId + reason).  
2. **Escrow status en API:** Devolver "LOCKED" en respuestas de escrow cuando el estado interno sea FUNDED, o documentar el mapeo FUNDED = locked para el frontend.  
3. **Order.displayStatus (opcional):** Añadir campo opcional en respuesta de Order que mapee status interno a la etiqueta que usa la UI (En progreso / Completada / Cancelada / En disputa).  
4. **Order.blockchain:** Si se persiste en creación, incluir en GET /orders/{id} y en schema Order.  
5. **GET /disputes (opcional):** Si más adelante la UI lista disputas, implementar GET /disputes (filtros por usuario, orden, status).

Con esto, la interfaz P2P queda alineada con el agregado Order, la FSM, el ledger y el procesador de depósitos, y se cubren los huecos detectados sin generar código frontend.
