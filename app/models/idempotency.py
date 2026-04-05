"""
Idempotency keys for order mutations and other events.
Ensures PUT /orders/{id}/accept, cancel, complete, dispute, mark-locked are executed at most once per key.
"""

from datetime import datetime, timezone
from uuid import uuid4

from sqlalchemy import Column, DateTime, Integer, String, Text

from app.db import Base


def _utc_now():
    return datetime.now(timezone.utc)


def _uuid():
    return str(uuid4())


class IdempotencyKeyModel(Base):
    """
    One row per processed idempotent operation.
    Client sends Idempotency-Key header (UUID). First request: INSERT then run command, store response.
    Duplicate request: return stored response (no duplicate state transitions / ledger / escrow).
    """

    __tablename__ = "idempotency_keys"

    id = Column(String(36), primary_key=True, default=_uuid)
    idempotency_key = Column(String(255), nullable=False, unique=True, index=True)
    order_id = Column(String(36), nullable=False, index=True)
    event_type = Column(String(64), nullable=False)  # legacy / deposit flow
    endpoint = Column(String(128), nullable=True)  # e.g. orders/accept, orders/complete
    result_snapshot = Column(Text(), nullable=True)  # JSON response body
    response_status = Column(Integer(), nullable=True)  # HTTP status; null = in progress
    response_hash = Column(String(64), nullable=True)  # optional hash of result_snapshot
    created_at = Column(DateTime, default=_utc_now)
