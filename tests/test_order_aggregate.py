"""
Tests del agregado Order (app.domain.order_aggregate) y OrderRepository.
Cubre: happy path, invariantes cruzadas, doble escrow, que escrow no se modifica fuera del aggregate.
"""

import os
import uuid
from datetime import datetime, timezone

import pytest
from fastapi import HTTPException
from sqlalchemy import create_engine, select
from sqlalchemy.orm import sessionmaker, Session

from app.db import Base
from app.domain.order_aggregate import OrderAggregate
from app.schemas.auth import UserResponse
from app.models import DepositEventModel, DomainEventModel, IdempotencyKeyModel, LedgerEntryModel  # noqa: F401 - register for create_all
from app.models.marketplace import EscrowModel, OrderModel
from app.repositories.order_repository import OrderRepository


@pytest.fixture
def db():
    """DB en memoria nueva por test: evita estado compartido y bloqueos FOR UPDATE entre tests."""
    engine = create_engine("sqlite:///:memory:", connect_args={"check_same_thread": False})
    Base.metadata.create_all(bind=engine)
    SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)
    session = SessionLocal()
    yield session
    session.close()


def _make_order(
    db: Session,
    status: str = "CREATED",
    buyer_id: str | None = None,
    escrow_id: str | None = None,
    seller_id: str = "seller-1",
    order_id: str | None = None,
) -> OrderModel:
    o = OrderModel(
        id=order_id or f"ord-{uuid.uuid4().hex[:12]}",
        seller_id=seller_id,
        buyer_id=buyer_id,
        seller_wallet="0xseller",
        buyer_wallet="0xbuyer" if buyer_id else None,
        crypto_currency="USDT",
        crypto_amount="100",
        fiat_currency="USD",
        fiat_amount="100",
        price_per_unit="1",
        status=status,
        escrow_id=escrow_id,
        payment_method="Wallet",
    )
    db.add(o)
    db.commit()
    db.refresh(o)
    return o


def _buyer_user_for_tests() -> UserResponse:
    """Mismo id/wallet que _make_order(..., buyer_id=\"b\")."""
    return UserResponse(
        id="b",
        walletAddress="0xbuyer",
        reputationScore=1.0,
        createdAt=datetime.now(timezone.utc).isoformat(),
    )


def _make_escrow(db: Session, order_id: str, status: str = "PENDING", escrow_id: str | None = None) -> EscrowModel:
    e = EscrowModel(
        id=escrow_id or f"esc-{uuid.uuid4().hex[:12]}",
        order_id=order_id,
        external_escrow_id=f"ext-{uuid.uuid4().hex[:8]}",
        contract_address="0xcontract",
        crypto_amount="100",
        crypto_currency="USDT",
        status=status,
    )
    db.add(e)
    db.commit()
    db.refresh(e)
    return e


# ----- Happy path -----

def test_aggregate_accept_then_fund_escrow_then_complete(db: Session):
    """Flujo: CREATED -> accept -> AWAITING_PAYMENT -> fund_escrow -> ESCROW_FUNDED -> complete -> RELEASED."""
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, status="CREATED", order_id=order_id)
    repo = OrderRepository(db)

    agg = repo.get_for_update(order_id)
    assert agg is not None
    assert agg.order.status == "CREATED"
    assert agg.escrow is None

    agg.accept("buyer-1", "0xbuyer", "0.9", "buyer-1")
    repo.save(agg)
    db.commit()
    db.refresh(agg.order)
    assert agg.order.status == "AWAITING_PAYMENT"
    assert agg.order.buyer_id == "buyer-1"

    agg2 = repo.get_for_update(order_id)
    assert agg2.order.status == "AWAITING_PAYMENT"
    agg2.fund_escrow(
        actor_id="buyer-1",
        external_escrow_id="ext-1",
        contract_address="0xc",
        crypto_amount="100",
        crypto_currency="USDT",
        create_tx_hash="0xtx",
    )
    repo.save(agg2)
    db.commit()
    db.refresh(agg2.order)
    if agg2.escrow:
        db.refresh(agg2.escrow)
    assert agg2.order.status == "ESCROW_FUNDED"
    assert agg2.escrow is not None
    assert agg2.escrow.status == "FUNDED"

    agg3 = repo.get_for_update(order_id)
    agg3.complete("SELLER", "seller-1")
    repo.save(agg3)
    db.commit()
    db.refresh(agg3.order)
    if agg3.escrow:
        db.refresh(agg3.escrow)
    assert agg3.order.status == "RELEASED"
    assert agg3.escrow.status == "RELEASED"


def test_aggregate_refund_from_awaiting(db: Session):
    """CREATED -> accept -> AWAITING_PAYMENT -> refund -> CANCELLED."""
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, status="CREATED", order_id=order_id)
    repo = OrderRepository(db)
    agg = repo.get_for_update(order_id)
    agg.accept("buyer-1", "0xb", "0.9", "buyer-1")
    repo.save(agg)
    db.commit()

    agg2 = repo.get_for_update(order_id)
    agg2.refund("SELLER", "SELLER", "seller-1")
    repo.save(agg2)
    db.commit()
    db.refresh(agg2.order)
    assert agg2.order.status == "CANCELLED"


def test_aggregate_open_dispute(db: Session):
    """AWAITING_PAYMENT -> open_dispute -> DISPUTED."""
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, status="AWAITING_PAYMENT", buyer_id="buyer-1", order_id=order_id)
    repo = OrderRepository(db)
    agg = repo.get_for_update(order_id)
    agg.open_dispute("BUYER", "buyer-1")
    repo.save(agg)
    db.commit()
    db.refresh(agg.order)
    assert agg.order.status == "DISPUTED"


# ----- Invariantes cruzadas -----

def test_attach_escrow_twice_raises(db: Session):
    """Solo un escrow por orden: segundo attach_escrow falla."""
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, status="AWAITING_PAYMENT", buyer_id="b", order_id=order_id)
    repo = OrderRepository(db)
    agg = repo.get_for_update(order_id)
    agg.attach_escrow("ext-1", "0xc", "100", "USDT", None)
    repo.save(agg)
    db.commit()

    agg2 = repo.get_for_update(order_id)
    with pytest.raises(HTTPException) as exc:
        agg2.attach_escrow("ext-2", "0xc2", "100", "USDT", None)
    assert exc.value.status_code == 409
    assert "already has an escrow" in exc.value.detail


def test_fund_escrow_without_escrow_requires_params(db: Session):
    """fund_escrow sin escrow existente y sin params debe fallar."""
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, status="AWAITING_PAYMENT", buyer_id="buyer-1", order_id=order_id)
    repo = OrderRepository(db)
    agg = repo.get_for_update(order_id)
    with pytest.raises(HTTPException) as exc:
        agg.fund_escrow("buyer-1")
    assert exc.value.status_code == 409
    assert "Escrow data required" in exc.value.detail


# ----- get_for_update y concurrencia -----

def test_get_for_update_returns_aggregate_with_order_and_escrow(db: Session):
    """get_for_update carga order + escrow bajo lock."""
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, status="AWAITING_PAYMENT", buyer_id="b", order_id=order_id)
    _make_escrow(db, order_id, status="FUNDED")
    o = db.get(OrderModel, order_id)
    o.escrow_id = db.scalar(select(EscrowModel).where(EscrowModel.order_id == order_id).limit(1)).id
    db.commit()

    repo = OrderRepository(db)
    agg = repo.get_for_update(order_id)
    assert agg is not None
    assert agg.order.id == order_id
    assert agg.escrow is not None
    assert agg.escrow.order_id == order_id


def test_get_for_update_none_when_order_missing(db: Session):
    repo = OrderRepository(db)
    agg = repo.get_for_update("nonexistent-id")
    assert agg is None


# ----- Resolve dispute -----

def test_resolve_dispute_release_from_disputed(db: Session):
    """DISPUTED + escrow FUNDED -> resolve_dispute_release -> order RELEASED, escrow RELEASED (sin divergencia)."""
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, status="DISPUTED", buyer_id="b", order_id=order_id)
    _make_escrow(db, order_id, status="FUNDED")
    o = db.get(OrderModel, order_id)
    e = db.scalar(select(EscrowModel).where(EscrowModel.order_id == order_id).limit(1))
    o.escrow_id = e.id
    db.commit()

    repo = OrderRepository(db)
    agg = repo.get_for_update(order_id)
    agg.resolve_dispute_release(release_tx_hash="0xrel", released_at="2025-01-01T00:00:00Z")
    repo.save(agg)
    db.commit()
    db.refresh(agg.order)
    db.refresh(agg.escrow)
    assert agg.order.status == "RELEASED"
    assert agg.escrow.status == "RELEASED"


def test_resolve_dispute_refund_from_disputed(db: Session):
    """DISPUTED + escrow FUNDED -> resolve_dispute_refund -> order CANCELLED, escrow REFUNDED (sin divergencia)."""
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, status="DISPUTED", buyer_id="b", order_id=order_id)
    _make_escrow(db, order_id, status="FUNDED")
    o = db.get(OrderModel, order_id)
    e = db.scalar(select(EscrowModel).where(EscrowModel.order_id == order_id).limit(1))
    o.escrow_id = e.id
    db.commit()

    repo = OrderRepository(db)
    agg = repo.get_for_update(order_id)
    agg.resolve_dispute_refund(refund_tx_hash="0xref", refunded_at="2025-01-01T00:00:00Z")
    repo.save(agg)
    db.commit()
    db.refresh(agg.order)
    db.refresh(agg.escrow)
    assert agg.order.status == "CANCELLED"
    assert agg.escrow.status == "REFUNDED"


def test_resolve_dispute_release_from_escrow_funded(db: Session):
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, status="ESCROW_FUNDED", buyer_id="b", order_id=order_id)
    _make_escrow(db, order_id, status="FUNDED")
    o = db.get(OrderModel, order_id)
    e = db.scalar(select(EscrowModel).where(EscrowModel.order_id == order_id).limit(1))
    o.escrow_id = e.id
    db.commit()

    repo = OrderRepository(db)
    agg = repo.get_for_update(order_id)
    agg.resolve_dispute_release(release_tx_hash="0xrel", released_at="2025-01-01T00:00:00Z")
    repo.save(agg)
    db.commit()
    db.refresh(agg.order)
    db.refresh(agg.escrow)
    assert agg.order.status == "RELEASED"
    assert agg.escrow.status == "RELEASED"


def test_resolve_dispute_refund_from_escrow_funded(db: Session):
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, status="ESCROW_FUNDED", buyer_id="b", order_id=order_id)
    _make_escrow(db, order_id, status="FUNDED")
    o = db.get(OrderModel, order_id)
    e = db.scalar(select(EscrowModel).where(EscrowModel.order_id == order_id).limit(1))
    o.escrow_id = e.id
    db.commit()

    repo = OrderRepository(db)
    agg = repo.get_for_update(order_id)
    agg.resolve_dispute_refund(refund_tx_hash="0xref", refunded_at="2025-01-01T00:00:00Z")
    repo.save(agg)
    db.commit()
    db.refresh(agg.order)
    db.refresh(agg.escrow)
    assert agg.order.status == "CANCELLED"
    assert agg.escrow.status == "REFUNDED"


# ----- Escrow no modificable fuera del aggregate -----

def test_escrow_only_updated_through_aggregate_methods(db: Session):
    """Verificación: los servicios no asignan EscrowModel.status directamente (solo vía aggregate)."""
    services_path = os.path.join(os.path.dirname(__file__), "..", "app", "services", "escrow.py")
    with open(services_path) as f:
        code = f.read()
    assert "m.status =" not in code
    assert "agg.resolve_dispute" in code or "agg.record_escrow" in code


def test_order_status_only_updated_via_aggregate(db: Session):
    """Verificación: orders.py usa aggregate para transiciones, no transition_order ni asignación directa."""
    orders_path = os.path.join(os.path.dirname(__file__), "..", "app", "services", "orders.py")
    with open(orders_path) as f:
        code = f.read()
    # No debe haber m.status = o order.status = en accept/mark/complete/cancel/dispute
    assert "transition_order" not in code
    assert "agg.accept" in code
    assert "agg.fund_escrow" in code
    assert "agg.complete" in code
    assert "agg.refund" in code
    assert "agg.open_dispute" in code


# ----- create_escrow vía aggregate (servicio) -----

def test_create_escrow_404_when_order_missing(db: Session):
    """create_escrow debe devolver 404 si la orden no existe."""
    from app.services.escrow import create_escrow
    with pytest.raises(HTTPException) as exc:
        create_escrow(
            db,
            user=_buyer_user_for_tests(),
            order_id="nonexistent-order-id",
            external_escrow_id="ext-1",
            contract_address="0xc",
            crypto_amount="100",
            crypto_currency="USDT",
            create_tx_hash=None,
        )
    assert exc.value.status_code == 404
    assert "Order not found" in exc.value.detail


def test_create_escrow_success_via_aggregate(db: Session):
    """create_escrow crea y enlaza escrow únicamente a través del aggregate."""
    from app.services.escrow import create_escrow, get_escrow_by_order_id
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, status="AWAITING_PAYMENT", buyer_id="b", order_id=order_id)
    result = create_escrow(
        db,
        user=_buyer_user_for_tests(),
        order_id=order_id,
        external_escrow_id="ext-123",
        contract_address="0xcontract",
        crypto_amount="100",
        crypto_currency="USDT",
        create_tx_hash=None,
    )
    assert result.orderId == order_id
    assert result.escrowId == "ext-123"
    assert result.status == "PENDING"
    # Verificar que la orden tiene escrow_id y que get_escrow_by_order_id lo encuentra
    escrow_by_order = get_escrow_by_order_id(db, order_id)
    assert escrow_by_order is not None
    assert escrow_by_order.id == result.id


def test_create_escrow_409_when_order_already_has_escrow(db: Session):
    """create_escrow debe fallar con 409 si la orden ya tiene un escrow."""
    from app.services.escrow import create_escrow
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, status="AWAITING_PAYMENT", buyer_id="b", order_id=order_id)
    _make_escrow(db, order_id, status="PENDING")
    o = db.get(OrderModel, order_id)
    e = db.scalar(select(EscrowModel).where(EscrowModel.order_id == order_id).limit(1))
    o.escrow_id = e.id
    db.commit()

    with pytest.raises(HTTPException) as exc:
        create_escrow(
            db,
            user=_buyer_user_for_tests(),
            order_id=order_id,
            external_escrow_id="ext-2",
            contract_address="0xc",
            crypto_amount="100",
            crypto_currency="USDT",
            create_tx_hash=None,
        )
    assert exc.value.status_code == 409
    assert "already has an escrow" in exc.value.detail


def test_domain_events_persisted_after_create_escrow(db: Session):
    """Tras create_escrow, los eventos de dominio se persisten en domain_events."""
    from app.models.domain_events import DomainEventModel
    from app.services.escrow import create_escrow
    from sqlalchemy import select
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, status="AWAITING_PAYMENT", buyer_id="b", order_id=order_id)
    create_escrow(
        db,
        user=_buyer_user_for_tests(),
        order_id=order_id,
        external_escrow_id="ext-1",
        contract_address="0xc",
        crypto_amount="100",
        crypto_currency="USDT",
        create_tx_hash=None,
    )
    rows = list(db.scalars(select(DomainEventModel).where(DomainEventModel.order_id == order_id)))
    assert len(rows) >= 1
    assert any(r.event_type == "EscrowAttached" for r in rows)


# ----- apply_deposit (idempotent deposit flow) -----

def test_apply_deposit_success(db: Session):
    """apply_deposit: AWAITING_PAYMENT + amount/currency match -> ESCROW_FUNDED, escrow FUNDED."""
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, status="AWAITING_PAYMENT", buyer_id="b", order_id=order_id)
    repo = OrderRepository(db)
    agg = repo.get_for_update(order_id)
    agg.apply_deposit(
        tx_hash="0xtx123",
        amount="100",
        currency="USDT",
        external_escrow_id="ext-1",
        contract_address="0xc",
    )
    repo.save(agg)
    db.commit()
    db.refresh(agg.order)
    if agg.escrow:
        db.refresh(agg.escrow)
    assert agg.order.status == "ESCROW_FUNDED"
    assert agg.escrow is not None
    assert agg.escrow.status == "FUNDED"
    assert agg.escrow.create_tx_hash == "0xtx123"


def test_apply_deposit_partial_rejected(db: Session):
    """Deposit with wrong amount or currency is rejected (partial deposit)."""
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, status="AWAITING_PAYMENT", buyer_id="b", order_id=order_id)
    repo = OrderRepository(db)
    agg = repo.get_for_update(order_id)
    with pytest.raises(HTTPException) as exc:
        agg.apply_deposit(
            tx_hash="0xtx",
            amount="50",
            currency="USDT",
            external_escrow_id="ext-1",
            contract_address="0xc",
        )
    assert exc.value.status_code == 409
    assert "amount" in (exc.value.detail or "").lower() or "currency" in (exc.value.detail or "").lower()


def test_deposit_processor_duplicate_webhook_idempotent(db: Session):
    """Same deposit webhook (same idempotency key) processed twice returns success both times, state applied once."""
    from app.services.deposit_processor import process_deposit_event, DepositEventPayload
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, status="AWAITING_PAYMENT", buyer_id="b", order_id=order_id)
    payload = DepositEventPayload(
        order_id=order_id,
        tx_hash="0xdup",
        amount="100",
        currency="USDT",
        external_escrow_id="ext-1",
        contract_address="0xc",
        idempotency_key="webhook-dup-1",
    )
    r1 = process_deposit_event(db, payload)
    assert r1.order_status == "ESCROW_FUNDED"
    assert r1.escrow_funded is True
    assert r1.already_processed is False

    r2 = process_deposit_event(db, payload)
    assert r2.escrow_funded is True
    assert r2.already_processed is True


def test_deposit_processor_idempotent_replay_same_tx(db: Session):
    """Replay of same tx_hash (no idempotency key) is idempotent when order already ESCROW_FUNDED with that tx."""
    from app.services.deposit_processor import process_deposit_event, DepositEventPayload
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, status="AWAITING_PAYMENT", buyer_id="b", order_id=order_id)
    payload = DepositEventPayload(
        order_id=order_id,
        tx_hash="0xreplay",
        amount="100",
        currency="USDT",
        external_escrow_id="ext-1",
        contract_address="0xc",
        idempotency_key=None,
    )
    process_deposit_event(db, payload)
    # Second call with same order_id+tx_hash (same derived key) -> already_processed
    r2 = process_deposit_event(db, payload)
    assert r2.already_processed is True


def test_deposit_processor_writes_to_outbox(db: Session):
    """After processing a deposit, domain event is written to outbox_events in same transaction."""
    from app.models.outbox import OutboxEventModel
    from app.services.deposit_processor import process_deposit_event, DepositEventPayload
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, status="AWAITING_PAYMENT", buyer_id="b", order_id=order_id)
    payload = DepositEventPayload(
        order_id=order_id,
        tx_hash="0xtx-outbox",
        amount="100",
        currency="USDT",
        external_escrow_id="ext-1",
        contract_address="0xc",
    )
    process_deposit_event(db, payload)
    outbox_rows = list(db.scalars(select(OutboxEventModel).where(OutboxEventModel.aggregate_id == order_id)))
    assert len(outbox_rows) >= 1
    assert any(r.event_type == "EscrowFunded" for r in outbox_rows)


def test_escrow_order_state_consistent_invariant(db: Session):
    """Aggregate rejects accept() when escrow is already FUNDED but order would be AWAITING_PAYMENT (cross-invariant)."""
    from app.domain.order_aggregate import OrderAggregate
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, status="CREATED", order_id=order_id)
    _make_escrow(db, order_id, status="FUNDED")  # Escrow FUNDED but order still CREATED = divergence
    o = db.get(OrderModel, order_id)
    e = db.scalar(select(EscrowModel).where(EscrowModel.order_id == order_id).limit(1))
    o.escrow_id = e.id
    db.commit()
    repo = OrderRepository(db)
    agg = repo.get_for_update(order_id)
    # accept() would set order to AWAITING_PAYMENT; cross-invariant: escrow FUNDED requires order ESCROW_FUNDED/DISPUTED
    with pytest.raises(HTTPException) as exc:
        agg.accept("buyer-1", "0xb", "0.9", "buyer-1")
    assert exc.value.status_code == 409
    assert "invariant" in (exc.value.detail or "").lower() or "FUNDED" in (exc.value.detail or "")


# ----- Idempotency under repeated calls (same key) -----

def test_concurrent_deposit_same_key_only_one_applies(db: Session):
    """Same deposit payload processed 5 times with same idempotency key: first applies, rest return already_processed."""
    from app.services.deposit_processor import process_deposit_event, DepositEventPayload

    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, status="AWAITING_PAYMENT", buyer_id="b", order_id=order_id)
    payload = DepositEventPayload(
        order_id=order_id,
        tx_hash="0xconcurrent",
        amount="100",
        currency="USDT",
        external_escrow_id="ext-1",
        contract_address="0xc",
        idempotency_key="concurrent-key-1",
    )
    results = []
    for _ in range(5):
        r = process_deposit_event(db, payload)
        results.append((r.order_status, r.already_processed))

    assert len(results) == 5
    applied = [r for r in results if not r[1]]
    already = [r for r in results if r[1]]
    assert len(applied) == 1, "Exactly one request should apply the deposit"
    assert len(already) == 4, "Four should get already_processed"
    assert applied[0][0] == "ESCROW_FUNDED"


# ----- Full flow integration -----

def test_integration_full_flow_create_accept_deposit_complete(db: Session):
    """End-to-end: create order, accept, process deposit, complete -> Order and Escrow never diverge."""
    from app.schemas.auth import UserResponse
    from app.services.deposit_processor import process_deposit_event, DepositEventPayload
    from app.services.orders import accept_order, complete_order, get_order_by_id

    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, status="CREATED", order_id=order_id, seller_id="seller-1")

    buyer = UserResponse(
        id="buyer-1",
        walletAddress="0xbuyer",
        reputationScore=0.9,
        isActive=True,
        loginCount=0,
        lastLoginAt=None,
        createdAt="2025-01-01T00:00:00Z",
    )
    seller = UserResponse(
        id="seller-1",
        walletAddress="0xseller",
        reputationScore=0.95,
        isActive=True,
        loginCount=0,
        lastLoginAt=None,
        createdAt="2025-01-01T00:00:00Z",
    )

    accept_order(db, order_id=order_id, buyer=buyer)
    order_after_accept = get_order_by_id(db, order_id)
    assert order_after_accept is not None
    assert order_after_accept.status == "AWAITING_PAYMENT"

    r = process_deposit_event(
        db,
        DepositEventPayload(
            order_id=order_id,
            tx_hash="0xfullflow",
            amount="100",
            currency="USDT",
            external_escrow_id="ext-full",
            contract_address="0xc",
        ),
    )
    assert r.order_status == "ESCROW_FUNDED"
    assert r.escrow_funded is True

    complete_order(db, order_id=order_id, user=seller)
    order_after_complete = get_order_by_id(db, order_id)
    assert order_after_complete is not None
    assert order_after_complete.status == "RELEASED"

    escrow = db.scalar(select(EscrowModel).where(EscrowModel.order_id == order_id).limit(1))
    assert escrow is not None
    assert escrow.status == "RELEASED"

    # Ledger: balances derived from entries; after deposit+release escrow=0, seller_balance=amount
    from app.services.ledger_service import get_balance
    from app.domain.ledger import ACCOUNT_ESCROW, ACCOUNT_SELLER_BALANCE
    assert get_balance(db, order_id, ACCOUNT_ESCROW, "USDT") == 0
    assert get_balance(db, order_id, ACCOUNT_SELLER_BALANCE, "USDT") == 100
    assert order_after_complete.status == "RELEASED"
