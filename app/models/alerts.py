"""
Alert model for persistent, per-user alerts.
Replaces the in-memory global list with database-backed storage.
"""

from datetime import datetime, timezone
from uuid import uuid4

from sqlalchemy import Boolean, Column, DateTime, Index, String, Text

from app.db import Base


def _utc_now():
    return datetime.now(timezone.utc)


def _uuid():
    return str(uuid4())


class AlertModel(Base):
    __tablename__ = "alerts"

    id = Column(String(36), primary_key=True, default=_uuid)
    user_id = Column(String(66), nullable=False, index=True)
    type = Column(String(32), nullable=False)
    title = Column(String(255), nullable=False)
    message = Column(Text(), nullable=False)
    severity = Column(String(16), nullable=False, default="medium")
    data = Column(Text(), nullable=True)
    read = Column(Boolean, nullable=False, default=False)
    created_at = Column(DateTime, default=_utc_now)

    __table_args__ = (
        Index("ix_alerts_user_read", "user_id", "read"),
        Index("ix_alerts_user_created", "user_id", "created_at"),
    )
