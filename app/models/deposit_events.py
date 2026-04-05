"""
Deposit events: incoming blockchain deposit events for deterministic processing.
One row per logical event (idempotency_key UNIQUE). Status: PENDING -> PROCESSED | REJECTED.
"""

from datetime import datetime, timezone
from uuid import uuid4

from sqlalchemy import Column, DateTime, String, Text, UniqueConstraint

from app.db import Base


def _utc_now():
    return datetime.now(timezone.utc)


def _uuid():
    return str(uuid4())


class DepositEventModel(Base):
    """
    Incoming deposit event from blockchain watcher.
    Idempotency: unique idempotency_key; first insert wins. Duplicate events return cached result.
    Además: UNIQUE (order_id, tx_hash) para no duplicar el mismo depósito on-chain.
    """

    __tablename__ = "deposit_events"
    __table_args__ = (UniqueConstraint("order_id", "tx_hash", name="uq_deposit_events_order_tx"),)

    id = Column(String(36), primary_key=True, default=_uuid)
    idempotency_key = Column(String(255), nullable=False, unique=True, index=True)
    order_id = Column(String(36), nullable=False, index=True)
    tx_hash = Column(String(66), nullable=False)
    amount = Column(String(64), nullable=False)
    currency = Column(String(20), nullable=False)
    external_escrow_id = Column(String(128), nullable=False)
    contract_address = Column(String(66), nullable=False)
    status = Column(String(32), nullable=False, default="PENDING", index=True)
    result_snapshot = Column(Text(), nullable=True)
    rejection_reason = Column(String(512), nullable=True)
    processed_at = Column(DateTime(), nullable=True)
    created_at = Column(DateTime(), default=_utc_now)


# Status values
DEPOSIT_EVENT_PENDING = "PENDING"
DEPOSIT_EVENT_PROCESSED = "PROCESSED"
DEPOSIT_EVENT_REJECTED = "REJECTED"
