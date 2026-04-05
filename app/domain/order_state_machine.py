"""
Strict state machine for Order (aggregate root) and Escrow.
Order states: CREATED | AWAITING_PAYMENT | PAID (= ESCROW_FUNDED) | ESCROW_FUNDED | RELEASED | CANCELLED | DISPUTED.
Escrow states: PENDING | FUNDED | RELEASED | REFUNDED.
Transitions enforced with guard conditions; invalid transitions raise InvalidTransitionError.
"""

from decimal import Decimal
from datetime import datetime, timezone
from enum import Enum
from typing import Any

from fastapi import HTTPException
from sqlalchemy import select
from sqlalchemy.orm import Session

from app.models.marketplace import DisputeModel, OrderModel

# ---------------------------------------------------------------------------
# Invalid transition exception (strict FSM)
# ---------------------------------------------------------------------------


class InvalidTransitionError(Exception):
    """Raised when a state transition is not allowed (guard failed or invalid from/to state)."""

    def __init__(self, message: str, from_state: str | None = None, to_state: str | None = None, event: str | None = None):
        self.from_state = from_state
        self.to_state = to_state
        self.event = event
        super().__init__(message)


# ---------------------------------------------------------------------------
# Order states
# ---------------------------------------------------------------------------

ORDER_STATES = frozenset({
    "CREATED",
    "AWAITING_PAYMENT",
    "PAID",            # Logical alias: transition to PAID requires escrow ledger balance == order amount
    "ESCROW_FUNDED",   # Stored in DB; PAID and ESCROW_FUNDED are the same logical state
    "RELEASED",
    "CANCELLED",
    "DISPUTED",
})

TERMINAL_STATES = frozenset({"RELEASED", "CANCELLED"})

# States that mean "order is paid" (escrow funded)
PAID_STATES = frozenset({"PAID", "ESCROW_FUNDED"})


class OrderEvent(str, Enum):
    BUYER_ACCEPT = "BUYER_ACCEPT"
    DEPOSIT_CONFIRMED = "DEPOSIT_CONFIRMED"
    MANUAL_MARK_FUNDED = "MANUAL_MARK_FUNDED"
    RELEASE = "RELEASE"
    CANCEL = "CANCEL"
    DISPUTE = "DISPUTE"
    RESOLVE_DISPUTE_RELEASE = "RESOLVE_DISPUTE_RELEASE"
    RESOLVE_DISPUTE_REFUND = "RESOLVE_DISPUTE_REFUND"


# (current_state, event) -> next_state
TRANSITION_MATRIX: dict[tuple[str, OrderEvent], str] = {
    ("CREATED", OrderEvent.BUYER_ACCEPT): "AWAITING_PAYMENT",
    ("CREATED", OrderEvent.CANCEL): "CANCELLED",
    ("AWAITING_PAYMENT", OrderEvent.DEPOSIT_CONFIRMED): "ESCROW_FUNDED",
    ("AWAITING_PAYMENT", OrderEvent.MANUAL_MARK_FUNDED): "ESCROW_FUNDED",
    ("AWAITING_PAYMENT", OrderEvent.CANCEL): "CANCELLED",
    ("AWAITING_PAYMENT", OrderEvent.DISPUTE): "DISPUTED",
    ("ESCROW_FUNDED", OrderEvent.RELEASE): "RELEASED",
    ("ESCROW_FUNDED", OrderEvent.CANCEL): "CANCELLED",
    ("ESCROW_FUNDED", OrderEvent.DISPUTE): "DISPUTED",
    ("DISPUTED", OrderEvent.RESOLVE_DISPUTE_RELEASE): "RELEASED",
    ("DISPUTED", OrderEvent.RESOLVE_DISPUTE_REFUND): "CANCELLED",
}

EVENT_ALLOWED_ROLES: dict[OrderEvent, frozenset[str]] = {
    OrderEvent.BUYER_ACCEPT: frozenset({"BUYER"}),
    OrderEvent.DEPOSIT_CONFIRMED: frozenset({"SYSTEM"}),
    OrderEvent.MANUAL_MARK_FUNDED: frozenset({"BUYER"}),
    OrderEvent.RELEASE: frozenset({"SELLER", "BUYER"}),
    OrderEvent.CANCEL: frozenset({"SELLER", "BUYER"}),
    OrderEvent.DISPUTE: frozenset({"SELLER", "BUYER"}),
    OrderEvent.RESOLVE_DISPUTE_RELEASE: frozenset({"SYSTEM"}),
    OrderEvent.RESOLVE_DISPUTE_REFUND: frozenset({"SYSTEM"}),
}

STATES_REQUIRING_BUYER = frozenset({"AWAITING_PAYMENT", "ESCROW_FUNDED", "RELEASED", "PAID"})
STATES_REQUIRING_ESCROW = frozenset({"ESCROW_FUNDED", "RELEASED", "PAID"})
DISPUTE_OPEN_STATUSES = frozenset({"OPEN", "IN_REVIEW", "ESCALATED"})

# ---------------------------------------------------------------------------
# Escrow states and transitions
# ---------------------------------------------------------------------------

ESCROW_STATES = frozenset({"PENDING", "FUNDED", "RELEASED", "REFUNDED"})
ESCROW_TERMINAL_STATES = frozenset({"RELEASED", "REFUNDED"})

# (from_state, to_state) allowed
ESCROW_TRANSITION_MATRIX = frozenset({
    ("PENDING", "FUNDED"),
    ("FUNDED", "RELEASED"),
    ("FUNDED", "REFUNDED"),
})

ESCROW_FUNDED = "FUNDED"
ESCROW_PENDING = "PENDING"
ESCROW_RELEASED = "RELEASED"
ESCROW_REFUNDED = "REFUNDED"


def validate_escrow_transition(from_state: str, to_state: str) -> bool:
    """True if escrow can transition from_state -> to_state."""
    if from_state not in ESCROW_STATES or to_state not in ESCROW_STATES:
        return False
    return (from_state, to_state) in ESCROW_TRANSITION_MATRIX


def guard_escrow_transition(from_state: str, to_state: str) -> None:
    """Raise InvalidTransitionError if escrow cannot transition from_state -> to_state."""
    if from_state in ESCROW_TERMINAL_STATES:
        raise InvalidTransitionError(
            f"Escrow in terminal state {from_state}; no transitions allowed",
            from_state=from_state,
            to_state=to_state,
        )
    if not validate_escrow_transition(from_state, to_state):
        raise InvalidTransitionError(
            f"Invalid escrow transition: {from_state} -> {to_state}",
            from_state=from_state,
            to_state=to_state,
        )


# ---------------------------------------------------------------------------
# Order transition guards (strict invariants)
# ---------------------------------------------------------------------------


def guard_order_can_transition_to_paid(
    order: OrderModel,
    escrow_balance: Decimal | None,
    order_amount: str,
    currency: str,
) -> None:
    """
    Order cannot transition to PAID/ESCROW_FUNDED unless escrow ledger balance equals order amount.
    Raises InvalidTransitionError if guard fails.
    """
    if escrow_balance is None:
        raise InvalidTransitionError(
            "Cannot transition to PAID: escrow ledger balance is required",
            to_state="ESCROW_FUNDED",
        )
    try:
        order_amt = Decimal(str(order_amount).strip())
    except Exception:
        raise InvalidTransitionError(
            "Cannot transition to PAID: invalid order amount",
            to_state="ESCROW_FUNDED",
        )
    if escrow_balance != order_amt:
        raise InvalidTransitionError(
            f"Cannot transition to PAID: escrow ledger balance ({escrow_balance}) must equal order amount ({order_amt})",
            to_state="ESCROW_FUNDED",
        )


def guard_order_can_transition_to_released(escrow_status: str | None) -> None:
    """
    Order cannot transition to RELEASED unless escrow state is FUNDED.
    Raises InvalidTransitionError if guard fails.
    """
    if escrow_status is None:
        raise InvalidTransitionError(
            "Cannot transition to RELEASED: escrow is required and must be FUNDED",
            to_state="RELEASED",
        )
    if escrow_status != ESCROW_FUNDED:
        raise InvalidTransitionError(
            f"Cannot transition to RELEASED: escrow state must be FUNDED, got {escrow_status}",
            to_state="RELEASED",
        )


def run_order_guards(
    order: OrderModel,
    event: OrderEvent,
    next_state: str,
    *,
    escrow_status: str | None = None,
    escrow_balance: Decimal | None = None,
) -> None:
    """
    Run all guard conditions for the given transition. Raises InvalidTransitionError if any guard fails.
    """
    if next_state in PAID_STATES and event in (OrderEvent.DEPOSIT_CONFIRMED, OrderEvent.MANUAL_MARK_FUNDED):
        guard_order_can_transition_to_paid(
            order,
            escrow_balance,
            order.crypto_amount,
            order.crypto_currency,
        )
    if next_state == "RELEASED" and event in (OrderEvent.RELEASE, OrderEvent.RESOLVE_DISPUTE_RELEASE):
        guard_order_can_transition_to_released(escrow_status)


def validate_transition(current_state: str, event: OrderEvent) -> str | None:
    if current_state in TERMINAL_STATES:
        return None
    return TRANSITION_MATRIX.get((current_state, event))


def validate_role(event: OrderEvent, actor_role: str) -> bool:
    return actor_role in EVENT_ALLOWED_ROLES.get(event, frozenset())


def _check_invariants(order: OrderModel, next_state: str, event: OrderEvent, db: Session) -> None:
    if next_state in STATES_REQUIRING_BUYER and not order.buyer_id:
        raise HTTPException(
            status_code=409,
            detail="Invalid transition: buyer_id is required for state " + next_state,
        )
    if next_state in STATES_REQUIRING_ESCROW and not order.escrow_id:
        raise HTTPException(
            status_code=409,
            detail="Invalid transition: escrow_id is required for state " + next_state,
        )
    if event == OrderEvent.DISPUTE and _count_open_disputes(db, order.id) > 0:
        raise HTTPException(
            status_code=409,
            detail="Invalid transition: order already has an open dispute",
        )


def _count_open_disputes(db: Session, order_id: str) -> int:
    from sqlalchemy import func
    n = db.scalar(
        select(func.count()).select_from(DisputeModel).where(
            DisputeModel.order_id == order_id,
            DisputeModel.status.in_(DISPUTE_OPEN_STATUSES),
        )
    )
    return int(n or 0)


def apply_transition(order: OrderModel, event: OrderEvent, **kwargs: Any) -> None:
    current = order.status
    next_state = validate_transition(current, event)
    if next_state is None:
        raise HTTPException(status_code=409, detail=f"Invalid transition from {current} with event {event.value}")

    now = datetime.now(timezone.utc)
    order.status = next_state

    if event == OrderEvent.BUYER_ACCEPT:
        order.accepted_at = now
    elif event in (OrderEvent.RELEASE, OrderEvent.RESOLVE_DISPUTE_RELEASE):
        order.completed_at = now
    elif event in (OrderEvent.CANCEL, OrderEvent.RESOLVE_DISPUTE_REFUND):
        order.cancelled_at = now
        if kwargs.get("cancelled_by"):
            order.cancelled_by = kwargs["cancelled_by"]
    elif event == OrderEvent.DISPUTE:
        order.disputed_at = now


def apply_transition_with_guards(
    order: OrderModel,
    event: OrderEvent,
    *,
    escrow_status: str | None = None,
    escrow_balance: Decimal | None = None,
    **kwargs: Any,
) -> None:
    """
    Run guard conditions for the transition, then apply. Raises InvalidTransitionError if guards fail.
    Pass escrow_balance when transitioning to PAID/ESCROW_FUNDED; pass escrow_status when transitioning to RELEASED.
    """
    next_state = validate_transition(order.status, event)
    if next_state is None:
        raise InvalidTransitionError(
            f"Invalid transition from {order.status} with event {event.value}",
            from_state=order.status,
            event=event.value,
        )
    run_order_guards(
        order,
        event,
        next_state,
        escrow_status=escrow_status,
        escrow_balance=escrow_balance,
    )
    apply_transition(order, event, **kwargs)


def transition_order(
    order: OrderModel,
    event: OrderEvent,
    actor_role: str,
    actor_id: str,
    db: Session,
    *,
    cancelled_by: str | None = None,
) -> OrderModel:
    if not validate_role(event, actor_role):
        raise HTTPException(status_code=403, detail=f"Role {actor_role} not allowed for event {event.value}")
    if event == OrderEvent.BUYER_ACCEPT and actor_id == order.seller_id:
        raise HTTPException(status_code=403, detail="Seller cannot accept their own order")
    if order.status in TERMINAL_STATES:
        raise HTTPException(
            status_code=409,
            detail=f"No transitions allowed from terminal state {order.status}",
        )
    next_state = validate_transition(order.status, event)
    if next_state is None:
        raise HTTPException(
            status_code=409,
            detail=f"Invalid transition from {order.status} with event {event.value}",
        )
    _check_invariants(order, next_state, event, db)
    apply_transition(order, event, cancelled_by=cancelled_by or "")
    db.commit()
    db.refresh(order)
    return order
