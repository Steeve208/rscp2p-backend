"""
Outbox table for reliable event publishing.
Events are written in the same transaction as domain changes; a worker publishes and marks processed.
Guarantees at-least-once delivery and no loss of events on commit.
"""

from datetime import datetime, timezone
from uuid import uuid4

from sqlalchemy import Column, DateTime, String, Text

from app.db import Base


def _utc_now():
    return datetime.now(timezone.utc)


def _uuid():
    return str(uuid4())


class OutboxEventModel(Base):
    """
    One row per domain event to be published.
    Written in same transaction as order/escrow updates; processed_at NULL until published.
    """

    __tablename__ = "outbox_events"

    id = Column(String(36), primary_key=True, default=_uuid)
    aggregate_type = Column(String(64), nullable=False, index=True)  # "Order"
    aggregate_id = Column(String(36), nullable=False, index=True)
    event_type = Column(String(64), nullable=False, index=True)
    payload = Column(Text(), nullable=False)
    created_at = Column(DateTime, default=_utc_now)
    processed_at = Column(DateTime, nullable=True)
