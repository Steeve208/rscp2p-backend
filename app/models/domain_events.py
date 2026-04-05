"""
Modelo para persistir eventos de dominio del aggregate Order (event log).
Permite auditoría, replay e integración asíncrona.
"""

from uuid import uuid4

from sqlalchemy import Column, DateTime, String, Text

from app.db import Base


def _utc_now():
    from datetime import datetime, timezone
    return datetime.now(timezone.utc)


def _uuid():
    return str(uuid4())


class DomainEventModel(Base):
    """Tabla domain_events: un registro por evento emitido por el aggregate."""

    __tablename__ = "domain_events"

    id = Column(String(36), primary_key=True, default=_uuid)
    order_id = Column(String(36), nullable=False, index=True)
    event_type = Column(String(64), nullable=False, index=True)  # payload["type"]
    payload = Column(Text(), nullable=False)  # JSON serializado (portable)
    occurred_at = Column(DateTime, nullable=False)
