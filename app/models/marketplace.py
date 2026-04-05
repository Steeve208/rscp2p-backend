"""
Modelos SQLAlchemy para Marketplace: orders, escrows, disputes.
Especificación: REESTRUCTURACION_PRODUCCION_MARKETPLACE_LAUNCHPAD.md (Migraciones MKT-001 a MKT-003).
"""

from datetime import datetime, timezone
from uuid import uuid4

from sqlalchemy import (
    Boolean,
    Column,
    DateTime,
    Float,
    ForeignKey,
    Index,
    Integer,
    Numeric,
    String,
    Text,
)
from sqlalchemy.orm import relationship

from app.db import Base


def _utc_now():
    return datetime.now(timezone.utc)


def _uuid():
    return str(uuid4())


class OrderModel(Base):
    """MKT-001: tabla orders."""

    __tablename__ = "orders"

    id = Column(String(36), primary_key=True, default=_uuid)
    seller_id = Column(String(64), nullable=False, index=True)
    buyer_id = Column(String(64), nullable=True, index=True)
    crypto_currency = Column(String(20), nullable=False)
    crypto_amount = Column(String(64), nullable=False)
    fiat_currency = Column(String(20), nullable=False)
    fiat_amount = Column(String(64), nullable=False)
    price_per_unit = Column(String(64), nullable=True)
    status = Column(String(32), nullable=False, index=True)
    payment_method = Column(String(120), nullable=True)
    terms = Column(Text(), nullable=True)
    expires_at = Column(DateTime, nullable=True)
    escrow_id = Column(String(36), nullable=True)
    accepted_at = Column(DateTime, nullable=True)
    completed_at = Column(DateTime, nullable=True)
    cancelled_at = Column(DateTime, nullable=True)
    cancelled_by = Column(String(16), nullable=True)
    disputed_at = Column(DateTime, nullable=True)
    created_at = Column(DateTime, default=_utc_now)
    updated_at = Column(DateTime, default=_utc_now, onupdate=_utc_now)

    # JSON o columnas para seller/buyer embebidos (opcional; se pueden reconstruir desde auth)
    seller_wallet = Column(String(66), nullable=True)
    seller_reputation = Column(String(16), nullable=True)
    buyer_wallet = Column(String(66), nullable=True)
    buyer_reputation = Column(String(16), nullable=True)

    __table_args__ = (
        Index("ix_orders_seller_created", "seller_id", "created_at"),
        Index("ix_orders_buyer_created", "buyer_id", "created_at"),
        Index("ix_orders_market_status", "crypto_currency", "fiat_currency", "status"),
    )

    escrow_rel = relationship(
        "EscrowModel",
        back_populates="order_rel",
        uselist=False,
        foreign_keys="EscrowModel.order_id",
    )


class EscrowModel(Base):
    """MKT-002: tabla escrows."""

    __tablename__ = "escrows"

    id = Column(String(36), primary_key=True, default=_uuid)
    order_id = Column(String(36), ForeignKey("orders.id"), nullable=False, unique=True, index=True)
    external_escrow_id = Column(String(128), nullable=False, unique=True, index=True)
    contract_address = Column(String(66), nullable=False)
    crypto_amount = Column(String(64), nullable=False)
    crypto_currency = Column(String(20), nullable=False)
    status = Column(String(32), nullable=False, default="PENDING")
    create_tx_hash = Column(String(66), nullable=True)
    release_tx_hash = Column(String(66), nullable=True)
    refund_tx_hash = Column(String(66), nullable=True)
    locked_at = Column(DateTime, nullable=True)
    released_at = Column(DateTime, nullable=True)
    refunded_at = Column(DateTime, nullable=True)
    created_at = Column(DateTime, default=_utc_now)
    updated_at = Column(DateTime, default=_utc_now, onupdate=_utc_now)

    __table_args__ = (
        Index("ix_escrows_status_updated", "status", "updated_at"),
    )

    order_rel = relationship("OrderModel", back_populates="escrow_rel", foreign_keys=[order_id])


class LedgerEntryModel(Base):
    """
    Append-only ledger entry for order escrow accounting.
    Balances are derived by summing entries; no balance columns stored.
    Accounts: buyer_balance, escrow, seller_balance.
    Double-entry: each transaction creates entries that sum to zero per currency.
    """

    __tablename__ = "ledger_entries"

    id = Column(String(36), primary_key=True, default=_uuid)
    order_id = Column(String(36), ForeignKey("orders.id", ondelete="CASCADE"), nullable=False, index=True)
    account = Column(String(32), nullable=False, index=True)
    amount = Column(Numeric(36, 18), nullable=False)
    currency = Column(String(20), nullable=False, index=True)
    type = Column(String(32), nullable=False, index=True)
    reference_id = Column(String(128), nullable=True)
    created_at = Column(DateTime, default=_utc_now)

    __table_args__ = (
        Index("ix_ledger_entries_order_account", "order_id", "account"),
        Index("ix_ledger_entries_order_created", "order_id", "created_at"),
    )


class DisputeModel(Base):
    """MKT-003: tabla disputes."""

    __tablename__ = "disputes"

    id = Column(String(36), primary_key=True, default=_uuid)
    order_id = Column(String(36), ForeignKey("orders.id"), nullable=False, index=True)
    initiator_id = Column(String(64), nullable=False)
    respondent_id = Column(String(64), nullable=True)
    reason = Column(Text(), nullable=True)
    status = Column(String(32), nullable=False, default="OPEN")
    resolution = Column(Text(), nullable=True)
    created_at = Column(DateTime, default=_utc_now)
    updated_at = Column(DateTime, default=_utc_now, onupdate=_utc_now)
    resolved_at = Column(DateTime, nullable=True)

    __table_args__ = (
        Index("ix_disputes_status_created", "status", "created_at"),
    )


class UserModel(Base):
    """MKT-004: tabla users (auth persistente)."""

    __tablename__ = "users"

    id = Column(String(36), primary_key=True, default=_uuid)
    wallet_address = Column(String(66), nullable=False, unique=True, index=True)
    reputation_score = Column(Float, nullable=False, default=0.0)
    is_active = Column(Boolean, nullable=False, default=True)
    login_count = Column(Integer, nullable=False, default=0)
    last_login_at = Column(DateTime, nullable=True)
    created_at = Column(DateTime, default=_utc_now)
    updated_at = Column(DateTime, default=_utc_now, onupdate=_utc_now)

    __table_args__ = (Index("ix_users_wallet", "wallet_address"),)
