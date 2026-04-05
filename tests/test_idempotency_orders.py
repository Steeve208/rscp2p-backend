"""
Tests for order mutation idempotency layer.
Same Idempotency-Key must return stored result without re-executing the command.
"""

import uuid
from datetime import datetime, timezone

import pytest
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker, Session

from app.db import Base
from app.models import (  # noqa: F401 - register for create_all
    DepositEventModel,
    DomainEventModel,
    IdempotencyKeyModel,
    LedgerEntryModel,
    OrderModel,
)
from app.models.marketplace import DisputeModel, EscrowModel, UserModel
from app.schemas.auth import UserResponse
from app.schemas.order import Order
from app.services.idempotency_service import run_idempotent, ENDPOINT_ACCEPT
from app.services.orders import accept_order, create_order


def _user_response(uid: str, wallet: str, score: float) -> UserResponse:
    return UserResponse(
        id=uid,
        walletAddress=wallet,
        reputationScore=score,
        createdAt=datetime.now(timezone.utc).isoformat(),
    )


@pytest.fixture
def db() -> Session:
    """Fresh in-memory DB per test."""
    engine = create_engine("sqlite:///:memory:", connect_args={"check_same_thread": False})
    Base.metadata.create_all(bind=engine)
    SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)
    session = SessionLocal()
    try:
        yield session
    finally:
        session.close()


@pytest.fixture
def seller_user(db: Session) -> UserModel:
    u = UserModel(
        id="seller-1",
        wallet_address="0xSeller12345678901234567890123456789012",
        reputation_score=85.0,
    )
    db.add(u)
    db.commit()
    db.refresh(u)
    return u


@pytest.fixture
def buyer_user(db: Session) -> UserModel:
    u = UserModel(
        id="buyer-1",
        wallet_address="0xBuyer123456789012345678901234567890123",
        reputation_score=70.0,
    )
    db.add(u)
    db.commit()
    db.refresh(u)
    return u


@pytest.fixture
def order_id(db: Session, seller_user: UserModel) -> str:
    seller = _user_response(
        seller_user.id,
        seller_user.wallet_address,
        float(seller_user.reputation_score or 0),
    )
    order = create_order(
        db,
        seller,
        crypto_currency="USDT",
        crypto_amount="100",
        fiat_currency="USD",
        fiat_amount="100",
        price_per_unit="1",
        payment_method="Wallet",
    )
    return order.id


def test_idempotent_accept_same_key_returns_cached_result(
    db: Session, order_id: str, buyer_user: UserModel
):
    """First request runs accept_order; second request with same key returns cached Order without re-running."""
    buyer = _user_response(
        buyer_user.id,
        buyer_user.wallet_address,
        float(buyer_user.reputation_score or 0),
    )
    key = str(uuid.uuid4())

    def run() -> Order:
        order = accept_order(db, order_id, buyer)
        if order is None:
            raise Exception("Order not found")
        return order

    status1, body1 = run_idempotent(
        db, key, ENDPOINT_ACCEPT, order_id, run, lambda o: o.model_dump(mode="json")
    )
    assert status1 == 200
    assert body1["id"] == order_id
    assert body1["buyerId"] == buyer_user.id
    assert body1["status"] == "AWAITING_PAYMENT"

    # Duplicate request: must return same result without running accept again
    status2, body2 = run_idempotent(
        db, key, ENDPOINT_ACCEPT, order_id, run, lambda o: o.model_dump(mode="json")
    )
    assert status2 == 200
    assert body2["id"] == body1["id"]
    assert body2["buyerId"] == body1["buyerId"]
    assert body2["status"] == body1["status"]

    # Only one row in idempotency_keys for this key
    count = db.query(IdempotencyKeyModel).filter(
        IdempotencyKeyModel.idempotency_key == key
    ).count()
    assert count == 1


def test_idempotent_accept_different_keys_create_two_rows(
    db: Session, order_id: str, buyer_user: UserModel
):
    """Two requests with different idempotency keys create two idempotency rows; each stores its result."""
    buyer = _user_response(buyer_user.id, buyer_user.wallet_address, 70.0)
    key1 = str(uuid.uuid4())
    key2 = str(uuid.uuid4())

    def run():
        from fastapi import HTTPException
        order = accept_order(db, order_id, buyer)
        if order is None:
            raise HTTPException(status_code=404, detail="Order not found")
        return order

    status1, _ = run_idempotent(
        db, key1, ENDPOINT_ACCEPT, order_id, run, lambda o: o.model_dump(mode="json")
    )
    assert status1 == 200

    # Second key runs again; aggregate may reject (already has buyer) or no-op. We store whatever result.
    status2, _ = run_idempotent(
        db, key2, ENDPOINT_ACCEPT, order_id, run, lambda o: o.model_dump(mode="json")
    )
    assert status2 in (200, 403, 409, 404)
    assert db.query(IdempotencyKeyModel).filter(
        IdempotencyKeyModel.idempotency_key == key1
    ).count() == 1
    assert db.query(IdempotencyKeyModel).filter(
        IdempotencyKeyModel.idempotency_key == key2
    ).count() == 1
