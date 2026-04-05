# Domain Event Log — Order Aggregate

Cada transición relevante del aggregate Order emite un **evento de dominio** que el servicio puede persistir para auditoría, replay o integración asíncrona.

## Eventos emitidos

| Evento            | Cuándo se emite                          |
|-------------------|-------------------------------------------|
| `OrderAccepted`   | Comprador acepta la orden (BUYER_ACCEPT).  |
| `EscrowAttached`  | Se enlaza o crea un escrow a la orden.    |
| `EscrowFunded`    | Escrow pasa a LOCKED (on-chain / metadata). |
| `EscrowReleased`  | Liberación a vendedor (COMPLETED).       |
| `EscrowRefunded`  | Reembolso a comprador (REFUNDED).        |
| `DisputeOpened`   | Se abre una disputa (DISPUTED).           |
| `DisputeResolved` | Resolución por liberación o reembolso (solo cuando la orden estaba DISPUTED). |

## Estructura de un evento

Cada evento es un `OrderDomainEvent` (dataclass inmutable):

- `order_id`: ID de la orden.
- `occurred_at`: Timestamp UTC del suceso.
- `payload`: dict con `type` y datos específicos (por ejemplo `buyer_id`, `escrow_id`, `release_tx_hash`).

Los eventos se **acumulan en memoria** en el aggregate y se extraen con `pull_domain_events()` después de `repo.save(agg)`.

## Uso en el servicio

```python
agg = repo.get_for_update(order_id)
agg.complete(role, user.id)
repo.save(agg)
events = agg.pull_domain_events()
db.commit()

# Persistir eventos (ejemplo: tabla domain_events o outbox)
for evt in events:
    persist_domain_event(evt)  # implementar según estrategia
```

## Persistencia (implementada)

Los servicios de órdenes y escrow llaman a `persist_domain_events(db, agg.pull_domain_events())` después de `repo.save(agg)` y antes de `db.commit()`, de modo que los eventos se escriben en la **misma transacción** que los cambios del aggregate.

- **Tabla** `domain_events`: columnas `id`, `order_id`, `event_type` (p. ej. `OrderAccepted`, `EscrowAttached`), `payload` (JSON en texto), `occurred_at`. Migración: `alembic/versions/002_domain_events.py`.
- **Modelo**: `app.models.domain_events.DomainEventModel`.
- **Helper**: `app.services.domain_events.persist_domain_events(db, events)`.

Opciones futuras: **outbox** (proceso asíncrono que publique a cola) o **solo logging** para trazabilidad sin BD.
