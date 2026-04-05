"""Auth nonces for challenge/verify. Persisted in DB for multi-instance support."""

from datetime import datetime, timezone

from sqlalchemy import Column, DateTime, String

from app.db import Base


def _utc_now():
    return datetime.now(timezone.utc)


class AuthNonceModel(Base):
    __tablename__ = "auth_nonces"

    wallet_address = Column(String(66), primary_key=True)
    nonce = Column(String(64), nullable=False)
    expires_at = Column(DateTime(), nullable=False)
    created_at = Column(DateTime(), default=_utc_now)
