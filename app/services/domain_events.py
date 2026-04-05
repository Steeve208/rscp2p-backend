"""
Persistencia de eventos de dominio emitidos por el aggregate Order.
Se llama después de repo.save(agg) y antes de db.commit() para incluir eventos en la misma transacción.
Outbox: escribe también en outbox_events para publicación asíncrona confiable.
"""

import json

from sqlalchemy.orm import Session

from app.domain.order_domain_events import OrderDomainEvent
from app.models.domain_events import DomainEventModel
from app.models.outbox import OutboxEventModel


def persist_domain_events(db: Session, events: list[OrderDomainEvent]) -> None:
    """Escribe los eventos en domain_events y en outbox_events (misma transacción). No hace commit."""
    for evt in events:
        event_type = evt.payload.get("type", "Unknown")
        payload_json = json.dumps(evt.payload)
        db.add(
            DomainEventModel(
                order_id=evt.order_id,
                event_type=event_type,
                payload=payload_json,
                occurred_at=evt.occurred_at,
            )
        )
        db.add(
            OutboxEventModel(
                aggregate_type="Order",
                aggregate_id=evt.order_id,
                event_type=event_type,
                payload=payload_json,
                created_at=evt.occurred_at,
            )
        )
