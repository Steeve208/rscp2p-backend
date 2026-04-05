"""
Tests unitarios de la FSM de órdenes (app.domain.order_state_machine).
Cubre transiciones válidas, inválidas, desde terminales, roles, invariantes y guardas estrictas.
"""

import uuid
from decimal import Decimal

import pytest
from fastapi import HTTPException
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker, Session

from app.db import Base
from app.domain.order_state_machine import (
    ORDER_STATES,
    TERMINAL_STATES,
    TRANSITION_MATRIX,
    EVENT_ALLOWED_ROLES,
    OrderEvent,
    InvalidTransitionError,
    validate_transition,
    validate_role,
    apply_transition,
    apply_transition_with_guards,
    transition_order,
    guard_order_can_transition_to_paid,
    guard_order_can_transition_to_released,
    guard_escrow_transition,
    validate_escrow_transition,
    ESCROW_STATES,
    ESCROW_FUNDED,
    ESCROW_PENDING,
    ESCROW_RELEASED,
    ESCROW_REFUNDED,
)
from app.models.marketplace import DisputeModel, OrderModel

# DB en memoria
_engine = create_engine("sqlite:///:memory:", connect_args={"check_same_thread": False})
_Session = sessionmaker(autocommit=False, autoflush=False, bind=_engine)


@pytest.fixture(scope="module")
def db():
    Base.metadata.create_all(bind=_engine)
    session = _Session()
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
        id=order_id or f"order-{uuid.uuid4().hex[:12]}",
        seller_id=seller_id,
        buyer_id=buyer_id,
        seller_wallet="0x111",
        buyer_wallet="0x222" if buyer_id else None,
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


# ----- Transiciones válidas -----

@pytest.mark.parametrize("current,event,next_state", [
    ("CREATED", OrderEvent.BUYER_ACCEPT, "AWAITING_PAYMENT"),
    ("CREATED", OrderEvent.CANCEL, "CANCELLED"),
    ("AWAITING_PAYMENT", OrderEvent.DEPOSIT_CONFIRMED, "ESCROW_FUNDED"),
    ("AWAITING_PAYMENT", OrderEvent.MANUAL_MARK_FUNDED, "ESCROW_FUNDED"),
    ("AWAITING_PAYMENT", OrderEvent.CANCEL, "CANCELLED"),
    ("AWAITING_PAYMENT", OrderEvent.DISPUTE, "DISPUTED"),
    ("ESCROW_FUNDED", OrderEvent.RELEASE, "RELEASED"),
    ("ESCROW_FUNDED", OrderEvent.CANCEL, "CANCELLED"),
    ("ESCROW_FUNDED", OrderEvent.DISPUTE, "DISPUTED"),
    ("DISPUTED", OrderEvent.RESOLVE_DISPUTE_RELEASE, "RELEASED"),
    ("DISPUTED", OrderEvent.RESOLVE_DISPUTE_REFUND, "CANCELLED"),
])
def test_validate_transition_valid(current: str, event: OrderEvent, next_state: str):
    assert validate_transition(current, event) == next_state


@pytest.mark.parametrize("current,event", [
    ("CREATED", OrderEvent.DEPOSIT_CONFIRMED),
    ("CREATED", OrderEvent.RELEASE),
    ("CREATED", OrderEvent.DISPUTE),
    ("AWAITING_PAYMENT", OrderEvent.BUYER_ACCEPT),
    ("AWAITING_PAYMENT", OrderEvent.RELEASE),
    ("ESCROW_FUNDED", OrderEvent.BUYER_ACCEPT),
    ("ESCROW_FUNDED", OrderEvent.DEPOSIT_CONFIRMED),
])
def test_validate_transition_invalid(current: str, event: OrderEvent):
    assert validate_transition(current, event) is None


@pytest.mark.parametrize("terminal", list(TERMINAL_STATES))
def test_no_transition_from_terminal(terminal: str):
    for event in OrderEvent:
        assert validate_transition(terminal, event) is None


# ----- Roles -----

@pytest.mark.parametrize("event,role", [
    (OrderEvent.BUYER_ACCEPT, "BUYER"),
    (OrderEvent.MANUAL_MARK_FUNDED, "BUYER"),
    (OrderEvent.RELEASE, "SELLER"),
    (OrderEvent.RELEASE, "BUYER"),
    (OrderEvent.CANCEL, "SELLER"),
    (OrderEvent.CANCEL, "BUYER"),
    (OrderEvent.DISPUTE, "SELLER"),
    (OrderEvent.DISPUTE, "BUYER"),
])
def test_validate_role_allowed(event: OrderEvent, role: str):
    assert validate_role(event, role) is True


@pytest.mark.parametrize("event,role", [
    (OrderEvent.BUYER_ACCEPT, "SELLER"),
    (OrderEvent.MANUAL_MARK_FUNDED, "SELLER"),
])
def test_validate_role_forbidden(event: OrderEvent, role: str):
    assert validate_role(event, role) is False


# ----- transition_order: transiciones válidas -----

def test_transition_order_buyer_accept(db: Session):
    order = _make_order(db, status="CREATED")
    order.buyer_id = "buyer-1"
    order.buyer_wallet = "0x222"
    db.commit()
    db.refresh(order)
    result = transition_order(order, OrderEvent.BUYER_ACCEPT, "BUYER", "buyer-1", db)
    assert result.status == "AWAITING_PAYMENT"
    assert result.accepted_at is not None


def test_transition_order_seller_cannot_accept(db: Session):
    order = _make_order(db, status="CREATED", seller_id="seller-1")
    order.buyer_id = None
    db.commit()
    db.refresh(order)
    with pytest.raises(HTTPException) as exc:
        transition_order(order, OrderEvent.BUYER_ACCEPT, "BUYER", "seller-1", db)
    assert exc.value.status_code == 403


def test_transition_order_manual_mark_funded(db: Session):
    order = _make_order(db, status="AWAITING_PAYMENT", buyer_id="buyer-1", escrow_id="escrow-1")
    result = transition_order(order, OrderEvent.MANUAL_MARK_FUNDED, "BUYER", "buyer-1", db)
    assert result.status == "ESCROW_FUNDED"


def test_transition_order_release(db: Session):
    order = _make_order(db, status="ESCROW_FUNDED", buyer_id="buyer-1", escrow_id="escrow-1")
    result = transition_order(order, OrderEvent.RELEASE, "SELLER", "seller-1", db)
    assert result.status == "RELEASED"
    assert result.completed_at is not None


def test_transition_order_cancel(db: Session):
    order = _make_order(db, status="CREATED")
    result = transition_order(order, OrderEvent.CANCEL, "SELLER", "seller-1", db, cancelled_by="SELLER")
    assert result.status == "CANCELLED"
    assert result.cancelled_at is not None
    assert result.cancelled_by == "SELLER"


def test_transition_order_dispute(db: Session):
    order = _make_order(db, status="AWAITING_PAYMENT", buyer_id="buyer-1")
    result = transition_order(order, OrderEvent.DISPUTE, "BUYER", "buyer-1", db)
    assert result.status == "DISPUTED"
    assert result.disputed_at is not None


# ----- transition_order: desde terminal -----

@pytest.mark.parametrize("terminal", ["RELEASED", "CANCELLED", "DISPUTED"])
def test_transition_order_from_terminal_rejected(db: Session, terminal: str):
    order = _make_order(db, status=terminal, buyer_id="buyer-1", escrow_id="e1" if terminal == "RELEASED" else None)
    with pytest.raises(HTTPException) as exc:
        transition_order(order, OrderEvent.CANCEL, "SELLER", "seller-1", db, cancelled_by="SELLER")
    assert exc.value.status_code == 409


# ----- transition_order: rol inválido -----

def test_transition_order_wrong_role_403(db: Session):
    order = _make_order(db, status="AWAITING_PAYMENT", buyer_id="buyer-1")
    with pytest.raises(HTTPException) as exc:
        transition_order(order, OrderEvent.MANUAL_MARK_FUNDED, "SELLER", "seller-1", db)
    assert exc.value.status_code == 403


# ----- transition_order: invariante escrow_id -----

def test_transition_order_escrow_required_for_funded(db: Session):
    order = _make_order(db, status="AWAITING_PAYMENT", buyer_id="buyer-1", escrow_id=None)
    with pytest.raises(HTTPException) as exc:
        transition_order(order, OrderEvent.MANUAL_MARK_FUNDED, "BUYER", "buyer-1", db)
    assert exc.value.status_code == 409
    assert "escrow_id" in (exc.value.detail or "").lower() or "escrow" in (exc.value.detail or "").lower()


# ----- transition_order: invariante buyer_id -----

def test_transition_order_buyer_required_for_awaiting(db: Session):
    order = _make_order(db, status="CREATED", buyer_id=None)
    # BUYER_ACCEPT sets buyer in the service before calling transition; here we call FSM directly
    # so buyer_id is still None. The invariant says AWAITING_FUNDS requires buyer_id.
    # When we apply BUYER_ACCEPT the next_state is AWAITING_FUNDS and we need buyer_id.
    # So we must set buyer_id on the order before calling transition_order for BUYER_ACCEPT,
    # which the service does. So the invariant check is: next_state in STATES_REQUIRING_BUYER -> buyer_id.
    # So if we call with order that has no buyer_id and event BUYER_ACCEPT, we're going to
    # next_state AWAITING_FUNDS, and order.buyer_id is None, so we raise 409.
    order.buyer_id = None
    db.commit()
    db.refresh(order)
    with pytest.raises(HTTPException) as exc:
        transition_order(order, OrderEvent.BUYER_ACCEPT, "BUYER", "buyer-1", db)
    assert exc.value.status_code == 409
    assert "buyer" in (exc.value.detail or "").lower()


# ----- apply_transition (unidad) -----

def test_apply_transition_updates_status_and_timestamps(db: Session):
    order = _make_order(db, status="CREATED")
    order.buyer_id = "b1"
    db.commit()
    db.refresh(order)
    apply_transition(order, OrderEvent.BUYER_ACCEPT)
    assert order.status == "AWAITING_PAYMENT"
    assert order.accepted_at is not None


def test_apply_transition_invalid_raises(db: Session):
    order = _make_order(db, status="CREATED")
    with pytest.raises(HTTPException) as exc:
        apply_transition(order, OrderEvent.DEPOSIT_CONFIRMED)
    assert exc.value.status_code == 409


# ----- Coherencia matriz / constantes -----

def test_all_matrix_states_in_order_states():
    for (s, _), next_s in TRANSITION_MATRIX.items():
        assert s in ORDER_STATES, s
        assert next_s in ORDER_STATES, next_s


def test_all_matrix_next_states_defined():
    for (s, e), next_s in TRANSITION_MATRIX.items():
        assert next_s in ORDER_STATES
        assert (s, e) in TRANSITION_MATRIX
        assert TRANSITION_MATRIX[(s, e)] == next_s


def test_terminal_states_subset_of_order_states():
    assert TERMINAL_STATES <= ORDER_STATES


def test_every_event_has_allowed_roles():
    for event in OrderEvent:
        assert event in EVENT_ALLOWED_ROLES
        assert len(EVENT_ALLOWED_ROLES[event]) >= 1


# ----- Strict FSM: InvalidTransitionError and guards -----

def test_guard_order_can_transition_to_paid_raises_when_balance_mismatch():
    """Order cannot transition to PAID unless escrow ledger balance equals order amount."""
    order = OrderModel(
        id="o1",
        seller_id="s1",
        buyer_id="b1",
        crypto_currency="USDT",
        crypto_amount="100",
        fiat_currency="USD",
        fiat_amount="100",
        status="AWAITING_PAYMENT",
    )
    with pytest.raises(InvalidTransitionError) as exc:
        guard_order_can_transition_to_paid(order, Decimal("50"), "100", "USDT")
    assert "balance" in str(exc.value).lower() or "amount" in str(exc.value).lower()
    assert exc.value.to_state == "ESCROW_FUNDED"

def test_guard_order_can_transition_to_paid_raises_when_balance_none():
    order = OrderModel(
        id="o1",
        seller_id="s1",
        buyer_id="b1",
        crypto_currency="USDT",
        crypto_amount="100",
        fiat_currency="USD",
        fiat_amount="100",
        status="AWAITING_PAYMENT",
    )
    with pytest.raises(InvalidTransitionError) as exc:
        guard_order_can_transition_to_paid(order, None, "100", "USDT")
    assert exc.value.to_state == "ESCROW_FUNDED"

def test_guard_order_can_transition_to_paid_passes_when_balance_matches():
    order = OrderModel(
        id="o1",
        seller_id="s1",
        buyer_id="b1",
        crypto_currency="USDT",
        crypto_amount="100",
        fiat_currency="USD",
        fiat_amount="100",
        status="AWAITING_PAYMENT",
    )
    guard_order_can_transition_to_paid(order, Decimal("100"), "100", "USDT")  # no raise


def test_guard_order_can_transition_to_released_raises_when_escrow_not_funded():
    """Order cannot transition to RELEASED unless escrow state is FUNDED."""
    with pytest.raises(InvalidTransitionError) as exc:
        guard_order_can_transition_to_released(ESCROW_PENDING)
    assert "FUNDED" in str(exc.value)
    assert exc.value.to_state == "RELEASED"

def test_guard_order_can_transition_to_released_raises_when_escrow_none():
    with pytest.raises(InvalidTransitionError) as exc:
        guard_order_can_transition_to_released(None)
    assert exc.value.to_state == "RELEASED"

def test_guard_order_can_transition_to_released_passes_when_escrow_funded():
    guard_order_can_transition_to_released(ESCROW_FUNDED)  # no raise


def test_guard_escrow_transition_valid():
    guard_escrow_transition(ESCROW_PENDING, ESCROW_FUNDED)
    guard_escrow_transition(ESCROW_FUNDED, ESCROW_RELEASED)
    guard_escrow_transition(ESCROW_FUNDED, ESCROW_REFUNDED)

def test_guard_escrow_transition_invalid_raises():
    with pytest.raises(InvalidTransitionError) as exc:
        guard_escrow_transition(ESCROW_PENDING, ESCROW_RELEASED)
    assert "Invalid escrow transition" in str(exc.value) or "PENDING" in str(exc.value)

def test_guard_escrow_transition_from_terminal_raises():
    with pytest.raises(InvalidTransitionError) as exc:
        guard_escrow_transition(ESCROW_RELEASED, ESCROW_FUNDED)
    assert "terminal" in str(exc.value).lower()


def test_validate_escrow_transition():
    assert validate_escrow_transition("PENDING", "FUNDED") is True
    assert validate_escrow_transition("FUNDED", "RELEASED") is True
    assert validate_escrow_transition("FUNDED", "REFUNDED") is True
    assert validate_escrow_transition("PENDING", "RELEASED") is False
    assert validate_escrow_transition("RELEASED", "FUNDED") is False


def test_apply_transition_with_guards_release_raises_when_escrow_not_funded(db: Session):
    """Transition to RELEASED with escrow PENDING raises InvalidTransitionError (guard)."""
    order = _make_order(db, status="ESCROW_FUNDED", buyer_id="b1", escrow_id="e1")
    # We need an escrow in PENDING to simulate invalid state (normally order ESCROW_FUNDED implies escrow FUNDED)
    from app.models.marketplace import EscrowModel
    escrow = EscrowModel(
        id="e1",
        order_id=order.id,
        external_escrow_id="ext1",
        contract_address="0xc",
        crypto_amount="100",
        crypto_currency="USDT",
        status=ESCROW_PENDING,  # not FUNDED
    )
    db.add(escrow)
    db.commit()
    db.refresh(order)
    with pytest.raises(InvalidTransitionError) as exc:
        apply_transition_with_guards(order, OrderEvent.RELEASE, escrow_status=ESCROW_PENDING)
    assert "FUNDED" in str(exc.value) or "RELEASED" in str(exc.value)
