"""
Order as Aggregate Root (DDD). Escrow is an internal entity; no direct Escrow updates outside this aggregate.
Production model: Order/Escrow mathematically consistent; deposit events idempotent via apply_deposit.
Domain invariants:
- Order cannot be PAID (ESCROW_FUNDED) without Escrow funded.
- Escrow cannot be RELEASED if Order not in RELEASED.
- Escrow cannot exist without Order.
- Escrow amount must equal Order amount.
- Deposit events processed exactly once (idempotency).
"""

from decimal import Decimal
from datetime import datetime, timezone
from uuid import uuid4

from fastapi import HTTPException

from app.domain.escrow_value_object import (
    ESCROW_FUNDED as ESCROW_FUNDED_VO,
    ESCROW_PENDING as ESCROW_PENDING_VO,
    ESCROW_RELEASED as ESCROW_RELEASED_VO,
    ESCROW_REFUNDED as ESCROW_REFUNDED_VO,
    EscrowValueObject,
)
from app.domain.order_domain_events import (
    dispute_opened as evt_dispute_opened,
    dispute_resolved as evt_dispute_resolved,
    escrow_attached as evt_escrow_attached,
    escrow_funded as evt_escrow_funded,
    escrow_refunded as evt_escrow_refunded,
    escrow_released as evt_escrow_released,
    order_accepted as evt_order_accepted,
)
from app.domain.order_state_machine import (
    OrderEvent,
    STATES_REQUIRING_BUYER,
    STATES_REQUIRING_ESCROW,
    TERMINAL_STATES,
    InvalidTransitionError,
    apply_transition,
    apply_transition_with_guards,
    guard_escrow_transition,
    validate_role,
    validate_transition,
)
from app.models.marketplace import EscrowModel, OrderModel

# Escrow status values (stored in DB; must match Order state invariants)
ESCROW_PENDING = ESCROW_PENDING_VO
ESCROW_FUNDED = ESCROW_FUNDED_VO
ESCROW_RELEASED = ESCROW_RELEASED_VO
ESCROW_REFUNDED = ESCROW_REFUNDED_VO


def _amounts_match(order_amount: str, order_currency: str, deposit_amount: str, deposit_currency: str) -> bool:
    """Normalized comparison for invariant: escrow amount must equal order amount."""
    if not order_amount or not deposit_amount:
        return False
    o_amount = order_amount.strip()
    d_amount = deposit_amount.strip()
    if o_amount != d_amount:
        return False
    if not order_currency or not deposit_currency:
        return True
    return order_currency.strip().upper() == deposit_currency.strip().upper()


class OrderAggregate:
    """
    Aggregate Root: Order. Escrow is internal; all Escrow state changes go through this aggregate.
    """

    def __init__(self, order: OrderModel, escrow: EscrowModel | None):
        self.order = order
        self.escrow = escrow
        self._domain_events: list = []

    def _emit(self, event) -> None:
        self._domain_events.append(event)

    def pull_domain_events(self) -> list:
        events, self._domain_events = self._domain_events[:], []
        return events

    def _validate_cross_invariants(self, next_order_status: str | None = None) -> None:
        o = self.order
        e = self.escrow
        status = next_order_status if next_order_status is not None else o.status

        if status in STATES_REQUIRING_ESCROW and not o.escrow_id:
            raise HTTPException(
                status_code=409,
                detail="Invalid transition: escrow_id is required for state " + status,
            )
        if status in STATES_REQUIRING_ESCROW and e is None:
            raise HTTPException(
                status_code=409,
                detail="Invalid transition: escrow entity required for state " + status,
            )
        if e is not None:
            # Invariant: escrow state must match order state (no divergence)
            if e.status == ESCROW_FUNDED and status not in ("ESCROW_FUNDED", "DISPUTED"):
                raise HTTPException(
                    status_code=409,
                    detail=f"Cross invariant: escrow FUNDED requires order ESCROW_FUNDED or DISPUTED, got {status}",
                )
            if e.status == ESCROW_RELEASED and status != "RELEASED":
                raise HTTPException(
                    status_code=409,
                    detail=f"Cross invariant: escrow RELEASED requires order RELEASED, got {status}",
                )
            if e.status == ESCROW_REFUNDED and status != "CANCELLED":
                raise HTTPException(
                    status_code=409,
                    detail=f"Cross invariant: escrow REFUNDED requires order CANCELLED, got {status}",
                )
            # Legal (escrow_status, order_status) pairs (no divergence)
            if not self._escrow_order_state_consistent(e.status, status):
                raise HTTPException(
                    status_code=409,
                    detail=f"Invalid escrow state {e.status} for order state {status}",
                )

    @staticmethod
    def _escrow_order_state_consistent(escrow_status: str, order_status: str) -> bool:
        """True if escrow status and order status are a valid pair (invariant: no divergence)."""
        valid_pairs = {
            (ESCROW_PENDING, "CREATED"),
            (ESCROW_PENDING, "AWAITING_PAYMENT"),
            (ESCROW_FUNDED, "ESCROW_FUNDED"),
            (ESCROW_FUNDED, "DISPUTED"),
            (ESCROW_RELEASED, "RELEASED"),
            (ESCROW_REFUNDED, "CANCELLED"),
        }
        return (escrow_status, order_status) in valid_pairs

    def _ensure_fsm_ok(self, event: OrderEvent, actor_role: str, actor_id: str) -> str:
        if not validate_role(event, actor_role):
            raise HTTPException(status_code=403, detail=f"Role {actor_role} not allowed for event {event.value}")
        if event == OrderEvent.BUYER_ACCEPT and actor_id == self.order.seller_id:
            raise HTTPException(status_code=403, detail="Seller cannot accept their own order")
        if self.order.status in TERMINAL_STATES:
            raise HTTPException(
                status_code=409,
                detail=f"No transitions allowed from terminal state {self.order.status}",
            )
        next_state = validate_transition(self.order.status, event)
        if next_state is None:
            raise HTTPException(
                status_code=409,
                detail=f"Invalid transition from {self.order.status} with event {event.value}",
            )
        if next_state in STATES_REQUIRING_BUYER and not self.order.buyer_id:
            raise HTTPException(
                status_code=409,
                detail="Invalid transition: buyer_id is required for state " + next_state,
            )
        return next_state

    def accept(self, buyer_id: str, buyer_wallet: str, buyer_reputation: str, actor_id: str) -> None:
        self.order.buyer_id = buyer_id
        self.order.buyer_wallet = buyer_wallet
        self.order.buyer_reputation = buyer_reputation
        self._ensure_fsm_ok(OrderEvent.BUYER_ACCEPT, "BUYER", actor_id)
        self._validate_cross_invariants("AWAITING_PAYMENT")
        apply_transition(self.order, OrderEvent.BUYER_ACCEPT)
        self._emit(evt_order_accepted(self.order.id, buyer_id, datetime.now(timezone.utc)))

    def _apply_with_guards_or_raise(self, event: OrderEvent, *, escrow_balance: Decimal | None = None, escrow_status: str | None = None, **kwargs) -> None:
        """Apply transition with guard checks; convert InvalidTransitionError to HTTP 409."""
        try:
            apply_transition_with_guards(
                self.order,
                event,
                escrow_balance=escrow_balance,
                escrow_status=escrow_status,
                **kwargs,
            )
        except InvalidTransitionError as e:
            raise HTTPException(status_code=409, detail=str(e))

    def apply_deposit(
        self,
        tx_hash: str,
        amount: str,
        currency: str,
        external_escrow_id: str,
        contract_address: str,
        escrow_balance: Decimal | None = None,
    ) -> None:
        """
        Idempotent deposit application (call from deposit processor after idempotency check).
        Transition AWAITING_PAYMENT -> ESCROW_FUNDED (PAID). Guard: escrow ledger balance must equal order amount.
        Creates escrow if not present; otherwise updates existing escrow to FUNDED.
        """
        if self.order.status != "AWAITING_PAYMENT":
            raise HTTPException(
                status_code=409,
                detail=f"Deposit only allowed in AWAITING_PAYMENT, current: {self.order.status}",
            )
        if not _amounts_match(self.order.crypto_amount, self.order.crypto_currency, amount, currency):
            raise HTTPException(
                status_code=409,
                detail="Deposit amount or currency does not match order (partial or wrong deposit)",
            )
        # Guard: transition to PAID requires escrow ledger balance == order amount
        balance = escrow_balance if escrow_balance is not None else Decimal(str(amount).strip())
        now = datetime.now(timezone.utc)
        if self.escrow is None:
            guard_escrow_transition("PENDING", ESCROW_FUNDED)
            e = EscrowModel(
                id=str(uuid4()),
                order_id=self.order.id,
                external_escrow_id=external_escrow_id,
                contract_address=contract_address,
                crypto_amount=self.order.crypto_amount,
                crypto_currency=self.order.crypto_currency,
                status=ESCROW_FUNDED,
                create_tx_hash=tx_hash,
                locked_at=now,
            )
            self.escrow = e
            self.order.escrow_id = e.id
        else:
            guard_escrow_transition(self.escrow.status, ESCROW_FUNDED)
            self.escrow.create_tx_hash = tx_hash
            self.escrow.crypto_amount = self.order.crypto_amount
            self.escrow.crypto_currency = self.order.crypto_currency
            self.escrow.status = ESCROW_FUNDED
            if self.escrow.locked_at is None:
                self.escrow.locked_at = now
        self._apply_with_guards_or_raise(OrderEvent.DEPOSIT_CONFIRMED, escrow_balance=balance)
        self._validate_cross_invariants()
        self._emit(evt_escrow_funded(
            self.order.id,
            self.escrow.id,
            tx_hash,
            now,
        ))

    def record_escrow_locked(self, create_tx_hash: str | None = None) -> None:
        """Metadata-only: set tx_hash and locked_at on escrow. Order must be ESCROW_FUNDED or DISPUTED."""
        if self.escrow is None:
            raise HTTPException(status_code=409, detail="No escrow to update")
        if self.order.status not in ("ESCROW_FUNDED", "DISPUTED"):
            raise HTTPException(status_code=409, detail="Order must be ESCROW_FUNDED or DISPUTED to record escrow locked")
        if create_tx_hash is not None:
            self.escrow.create_tx_hash = create_tx_hash
            if self.escrow.locked_at is None:
                self.escrow.locked_at = datetime.now(timezone.utc)
        self.escrow.status = ESCROW_FUNDED
        self._validate_cross_invariants()
        self._emit(evt_escrow_funded(self.order.id, self.escrow.id, create_tx_hash, datetime.now(timezone.utc)))

    def link_escrow(self, escrow: EscrowModel) -> None:
        if escrow.order_id != self.order.id:
            raise HTTPException(status_code=409, detail="Escrow does not belong to this order")
        if self.escrow is not None:
            raise HTTPException(status_code=409, detail="Order already has an escrow")
        self.escrow = escrow
        self.order.escrow_id = escrow.id
        self._validate_cross_invariants()
        self._emit(evt_escrow_attached(self.order.id, escrow.id, datetime.now(timezone.utc)))

    def attach_escrow(
        self,
        external_escrow_id: str,
        contract_address: str,
        crypto_amount: str,
        crypto_currency: str,
        create_tx_hash: str | None = None,
    ) -> None:
        if self.escrow is not None:
            raise HTTPException(status_code=409, detail="Order already has an escrow")
        now = datetime.now(timezone.utc)
        e = EscrowModel(
            id=str(uuid4()),
            order_id=self.order.id,
            external_escrow_id=external_escrow_id,
            contract_address=contract_address,
            crypto_amount=crypto_amount,
            crypto_currency=crypto_currency,
            status=ESCROW_FUNDED if create_tx_hash else ESCROW_PENDING,
            create_tx_hash=create_tx_hash,
            locked_at=now if create_tx_hash else None,
        )
        self.escrow = e
        self.order.escrow_id = e.id
        self._validate_cross_invariants()
        self._emit(evt_escrow_attached(self.order.id, e.id, now))

    def fund_escrow(
        self,
        actor_id: str,
        external_escrow_id: str | None = None,
        contract_address: str | None = None,
        crypto_amount: str | None = None,
        crypto_currency: str | None = None,
        create_tx_hash: str | None = None,
    ) -> None:
        """Manual mark as funded (no webhook). Uses MANUAL_MARK_FUNDED; guard: escrow balance = order amount."""
        self._ensure_fsm_ok(OrderEvent.MANUAL_MARK_FUNDED, "BUYER", actor_id)
        now = datetime.now(timezone.utc)
        if self.escrow is None:
            if not all((external_escrow_id, contract_address, crypto_amount, crypto_currency)):
                raise HTTPException(status_code=409, detail="Escrow data required when order has no escrow")
            guard_escrow_transition(ESCROW_PENDING, ESCROW_FUNDED)
            e = EscrowModel(
                id=str(uuid4()),
                order_id=self.order.id,
                external_escrow_id=external_escrow_id,
                contract_address=contract_address,
                crypto_amount=crypto_amount,
                crypto_currency=crypto_currency,
                status=ESCROW_FUNDED if create_tx_hash else ESCROW_PENDING,
                create_tx_hash=create_tx_hash,
                locked_at=now if create_tx_hash else None,
            )
            self.escrow = e
            self.order.escrow_id = e.id
        else:
            if create_tx_hash:
                guard_escrow_transition(self.escrow.status, ESCROW_FUNDED)
                self.escrow.create_tx_hash = create_tx_hash
                self.escrow.status = ESCROW_FUNDED
                if self.escrow.locked_at is None:
                    self.escrow.locked_at = now
        self._validate_cross_invariants("ESCROW_FUNDED")
        escrow_balance = Decimal(str((crypto_amount or self.order.crypto_amount)).strip())
        self._apply_with_guards_or_raise(OrderEvent.MANUAL_MARK_FUNDED, escrow_balance=escrow_balance)
        self._emit(evt_escrow_funded(
            self.order.id,
            self.escrow.id if self.escrow else None,
            create_tx_hash,
            now,
        ))

    def complete(self, actor_role: str, actor_id: str) -> None:
        self._ensure_fsm_ok(OrderEvent.RELEASE, actor_role, actor_id)
        # Guard: transition to RELEASED requires escrow state FUNDED
        escrow_status = self.escrow.status if self.escrow is not None else None
        self._apply_with_guards_or_raise(OrderEvent.RELEASE, escrow_status=escrow_status)
        if self.escrow is not None:
            guard_escrow_transition(self.escrow.status, ESCROW_RELEASED)
            self.escrow.status = ESCROW_RELEASED
        self._validate_cross_invariants()
        self._emit(evt_escrow_released(
            self.order.id,
            self.escrow.id if self.escrow else None,
            None,
            datetime.now(timezone.utc),
        ))

    def refund(self, cancelled_by: str, actor_role: str, actor_id: str) -> None:
        self._ensure_fsm_ok(OrderEvent.CANCEL, actor_role, actor_id)
        apply_transition(self.order, OrderEvent.CANCEL, cancelled_by=cancelled_by)
        if self.escrow is not None:
            guard_escrow_transition(self.escrow.status, ESCROW_REFUNDED)
            self.escrow.status = ESCROW_REFUNDED
        self._validate_cross_invariants()
        self._emit(evt_escrow_refunded(
            self.order.id,
            self.escrow.id if self.escrow else None,
            None,
            datetime.now(timezone.utc),
        ))

    def open_dispute(self, actor_role: str, actor_id: str) -> None:
        self._ensure_fsm_ok(OrderEvent.DISPUTE, actor_role, actor_id)
        self._validate_cross_invariants("DISPUTED")
        apply_transition(self.order, OrderEvent.DISPUTE)
        self._emit(evt_dispute_opened(self.order.id, datetime.now(timezone.utc)))

    def resolve_dispute_release(
        self,
        release_tx_hash: str | None = None,
        released_at: str | None = None,
    ) -> None:
        was_disputed = self.order.status == "DISPUTED"
        if self.order.status == "ESCROW_FUNDED":
            escrow_status = self.escrow.status if self.escrow else None
            self._apply_with_guards_or_raise(OrderEvent.RELEASE, escrow_status=escrow_status)
            if self.escrow is not None:
                guard_escrow_transition(self.escrow.status, ESCROW_RELEASED)
                self.escrow.status = ESCROW_RELEASED
                if release_tx_hash:
                    self.escrow.release_tx_hash = release_tx_hash
                if released_at:
                    try:
                        self.escrow.released_at = datetime.fromisoformat(released_at.replace("Z", "+00:00"))
                    except (ValueError, TypeError):
                        pass
        elif self.order.status == "DISPUTED" and self.escrow is not None:
            self._apply_with_guards_or_raise(OrderEvent.RESOLVE_DISPUTE_RELEASE, escrow_status=self.escrow.status)
            guard_escrow_transition(self.escrow.status, ESCROW_RELEASED)
            self.escrow.status = ESCROW_RELEASED
            if release_tx_hash:
                self.escrow.release_tx_hash = release_tx_hash
            if released_at:
                try:
                    self.escrow.released_at = datetime.fromisoformat(released_at.replace("Z", "+00:00"))
                except (ValueError, TypeError):
                    pass
        else:
            raise HTTPException(
                status_code=409,
                detail="resolve_dispute_release not applicable for current order/escrow state",
            )
        self._validate_cross_invariants()
        now = datetime.now(timezone.utc)
        self._emit(evt_escrow_released(
            self.order.id,
            self.escrow.id if self.escrow else None,
            release_tx_hash,
            now,
        ))
        if was_disputed:
            self._emit(evt_dispute_resolved(self.order.id, "release", now, release_tx_hash=release_tx_hash))

    def resolve_dispute_refund(
        self,
        refund_tx_hash: str | None = None,
        refunded_at: str | None = None,
    ) -> None:
        was_disputed = self.order.status == "DISPUTED"
        if self.order.status == "ESCROW_FUNDED":
            apply_transition(self.order, OrderEvent.CANCEL, cancelled_by="")
            if self.escrow is not None:
                guard_escrow_transition(self.escrow.status, ESCROW_REFUNDED)
                self.escrow.status = ESCROW_REFUNDED
                if refund_tx_hash:
                    self.escrow.refund_tx_hash = refund_tx_hash
                if refunded_at:
                    try:
                        self.escrow.refunded_at = datetime.fromisoformat(refunded_at.replace("Z", "+00:00"))
                    except (ValueError, TypeError):
                        pass
        elif self.order.status == "DISPUTED" and self.escrow is not None:
            self._apply_with_guards_or_raise(OrderEvent.RESOLVE_DISPUTE_REFUND, escrow_status=self.escrow.status)
            guard_escrow_transition(self.escrow.status, ESCROW_REFUNDED)
            self.escrow.status = ESCROW_REFUNDED
            if refund_tx_hash:
                self.escrow.refund_tx_hash = refund_tx_hash
            if refunded_at:
                try:
                    self.escrow.refunded_at = datetime.fromisoformat(refunded_at.replace("Z", "+00:00"))
                except (ValueError, TypeError):
                    pass
        else:
            raise HTTPException(
                status_code=409,
                detail="resolve_dispute_refund not applicable for current order/escrow state",
            )
        self._validate_cross_invariants()
        now = datetime.now(timezone.utc)
        self._emit(evt_escrow_refunded(
            self.order.id,
            self.escrow.id if self.escrow else None,
            refund_tx_hash,
            now,
        ))
        if was_disputed:
            self._emit(evt_dispute_resolved(self.order.id, "refund", now, refund_tx_hash=refund_tx_hash))
