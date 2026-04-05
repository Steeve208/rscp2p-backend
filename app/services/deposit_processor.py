"""
Deterministic deposit processing pipeline.

Flow: DepositEvent → Idempotency Guard (deposit_events) → Order Aggregate (row lock)
      → Ledger Entry → Escrow State Update → Domain Event.

Single transaction: BEGIN → insert/claim deposit_event → lock order → validate
→ ledger → escrow update → domain event → update deposit_event status → COMMIT.

Deposits are processed exactly once (unique idempotency_key on deposit_events).
"""

from datetime import datetime, timezone
from decimal import Decimal
from uuid import uuid4

from fastapi import HTTPException
from sqlalchemy import select
from sqlalchemy.exc import IntegrityError
from sqlalchemy.orm import Session

from app.domain.order_aggregate import OrderAggregate
from app.models.deposit_events import (
    DEPOSIT_EVENT_PENDING,
    DEPOSIT_EVENT_PROCESSED,
    DEPOSIT_EVENT_REJECTED,
    DepositEventModel,
)
from app.repositories.order_repository import OrderRepository
from app.services.domain_events import persist_domain_events
from app.services.ledger_service import create_balanced_entries, entries_for_deposit


# --- Event schema (API / blockchain watcher) ---

class DepositEventPayload:
    """Incoming deposit event from BlockchainWatcher. Immutable payload."""

    __slots__ = ("order_id", "tx_hash", "amount", "currency", "external_escrow_id", "contract_address", "idempotency_key")

    def __init__(
        self,
        order_id: str,
        tx_hash: str,
        amount: str,
        currency: str,
        external_escrow_id: str,
        contract_address: str,
        idempotency_key: str | None = None,
    ):
        self.order_id = order_id
        self.tx_hash = tx_hash
        self.amount = amount
        self.currency = currency
        self.external_escrow_id = external_escrow_id
        self.contract_address = contract_address
        self.idempotency_key = idempotency_key


class DepositResult:
    """Result of processing a deposit event."""

    __slots__ = ("order_id", "order_status", "escrow_funded", "already_processed", "rejected", "rejection_reason")

    def __init__(
        self,
        order_id: str,
        order_status: str,
        escrow_funded: bool,
        already_processed: bool,
        rejected: bool = False,
        rejection_reason: str | None = None,
    ):
        self.order_id = order_id
        self.order_status = order_status
        self.escrow_funded = escrow_funded
        self.already_processed = already_processed
        self.rejected = rejected
        self.rejection_reason = rejection_reason


def _idempotency_key(payload: DepositEventPayload) -> str:
    if payload.idempotency_key and payload.idempotency_key.strip():
        return payload.idempotency_key.strip()
    return f"deposit:{payload.order_id}:{payload.tx_hash}"


def _parse_result_snapshot(snapshot: str | None) -> tuple[str, bool]:
    """Parse result_snapshot (order_status). Returns (order_status, escrow_funded)."""
    if not snapshot:
        return "ESCROW_FUNDED", True
    return snapshot.strip(), True


def _validate_amount(
    order_amount: str,
    order_currency: str,
    deposit_amount: str,
    deposit_currency: str,
) -> tuple[bool, str | None]:
    """
    Validate deposit amount/currency. Returns (ok, rejection_reason).
    Rejects: partial (amount < order), overpayment (amount > order), currency mismatch.
    """
    if not order_amount or not deposit_amount:
        return False, "Missing amount"
    try:
        order_amt = Decimal(str(order_amount).strip())
        dep_amt = Decimal(str(deposit_amount).strip())
    except Exception:
        return False, "Invalid amount format"
    if dep_amt <= 0:
        return False, "Deposit amount must be positive"
    if dep_amt < order_amt:
        return False, "Partial deposit not allowed"
    if dep_amt > order_amt:
        return False, "Overpayment not allowed"
    if order_currency and deposit_currency:
        if order_currency.strip().upper() != deposit_currency.strip().upper():
            return False, "Currency mismatch"
    return True, None


def _claim_deposit_event(db: Session, key: str, payload: DepositEventPayload) -> DepositEventModel | None:
    """
    Claim idempotency by inserting a deposit_events row (PENDING).
    Returns the new row if we claimed; None if duplicate (already exists).
    """
    try:
        row = DepositEventModel(
            id=str(uuid4()),
            idempotency_key=key,
            order_id=payload.order_id,
            tx_hash=payload.tx_hash,
            amount=payload.amount,
            currency=payload.currency,
            external_escrow_id=payload.external_escrow_id,
            contract_address=payload.contract_address,
            status=DEPOSIT_EVENT_PENDING,
        )
        db.add(row)
        db.flush()
        return row
    except IntegrityError:
        db.rollback()
        return None


def _row_to_deposit_result(row: DepositEventModel) -> DepositResult:
    """Interpreta una fila deposit_events ya persistida (por key o por order_id+tx_hash)."""
    if row.status == DEPOSIT_EVENT_REJECTED:
        return DepositResult(
            order_id=row.order_id,
            order_status=row.result_snapshot or "AWAITING_PAYMENT",
            escrow_funded=False,
            already_processed=True,
            rejected=True,
            rejection_reason=row.rejection_reason or "Rejected",
        )
    status, escrow_funded = _parse_result_snapshot(row.result_snapshot)
    return DepositResult(
        order_id=row.order_id,
        order_status=status,
        escrow_funded=escrow_funded,
        already_processed=True,
        rejected=False,
    )


def _get_cached_result(db: Session, key: str) -> DepositResult | None:
    """Return cached result for an already-processed or rejected deposit event."""
    row = db.scalar(
        select(DepositEventModel).where(DepositEventModel.idempotency_key == key).limit(1)
    )
    if row is None:
        return None
    return _row_to_deposit_result(row)


def _get_cached_result_by_order_tx(db: Session, order_id: str, tx_hash: str) -> DepositResult | None:
    """Mismo depósito on-chain con distinta idempotency_key: UNIQUE (order_id, tx_hash)."""
    row = db.scalar(
        select(DepositEventModel).where(
            DepositEventModel.order_id == order_id,
            DepositEventModel.tx_hash == tx_hash,
        ).limit(1)
    )
    if row is None:
        return None
    return _row_to_deposit_result(row)


def process_deposit_event(db: Session, payload: DepositEventPayload) -> DepositResult:
    """
    Process a blockchain deposit event exactly once in a single transaction.

    Transaction boundaries:
    1. INSERT deposit_events (idempotency_key UNIQUE) → claim. If duplicate, return cached.
    2. SELECT order FOR UPDATE (row-level lock).
    3. Validate order exists and status AWAITING_PAYMENT.
    4. Validate amount (reject partial deposit and overpayment).
    5. Apply ledger entries (double-entry).
    6. Order aggregate: apply_deposit (escrow state update).
    7. Persist domain events (outbox).
    8. UPDATE deposit_events SET status=PROCESSED, result_snapshot, processed_at.
    9. COMMIT.

    Concurrency: first request to insert deposit_events wins; others get IntegrityError and return cached.
    """
    key = _idempotency_key(payload)
    now = datetime.now(timezone.utc)

    # Idempotency guard: claim via deposit_events
    event_row = _claim_deposit_event(db, key, payload)
    if event_row is None:
        cached = _get_cached_result(db, key)
        if cached is not None:
            return cached
        cached_tx = _get_cached_result_by_order_tx(db, payload.order_id, payload.tx_hash)
        if cached_tx is not None:
            return cached_tx
        return DepositResult(
            order_id=payload.order_id,
            order_status="ESCROW_FUNDED",
            escrow_funded=True,
            already_processed=True,
            rejected=False,
        )

    repo = OrderRepository(db)
    agg = repo.get_for_update(payload.order_id)
    if agg is None:
        event_row.status = DEPOSIT_EVENT_REJECTED
        event_row.rejection_reason = "Order not found"
        event_row.result_snapshot = None
        event_row.processed_at = now
        db.commit()
        raise HTTPException(status_code=404, detail="Order not found")

    # Replay: order already funded with same tx → mark processed and commit
    if (
        agg.order.status == "ESCROW_FUNDED"
        and agg.escrow is not None
        and agg.escrow.create_tx_hash == payload.tx_hash
    ):
        event_row.status = DEPOSIT_EVENT_PROCESSED
        event_row.result_snapshot = agg.order.status
        event_row.processed_at = now
        db.commit()
        db.refresh(agg.order)
        if agg.escrow:
            db.refresh(agg.escrow)
        return DepositResult(
            order_id=payload.order_id,
            order_status=agg.order.status,
            escrow_funded=True,
            already_processed=True,
            rejected=False,
        )

    # Validate: partial deposit and overpayment prevention
    ok, rejection_reason = _validate_amount(
        agg.order.crypto_amount,
        agg.order.crypto_currency,
        payload.amount,
        payload.currency,
    )
    if not ok:
        event_row.status = DEPOSIT_EVENT_REJECTED
        event_row.rejection_reason = rejection_reason
        event_row.result_snapshot = agg.order.status
        event_row.processed_at = now
        db.commit()
        raise HTTPException(
            status_code=409,
            detail=rejection_reason or "Deposit amount or currency does not match order",
        )

    # Ledger entries (double-entry)
    deposit_entries = entries_for_deposit(
        agg.order.id, payload.amount, payload.currency, payload.tx_hash
    )
    create_balanced_entries(db, agg.order.id, deposit_entries)

    # Escrow state update (Order aggregate)
    agg.apply_deposit(
        tx_hash=payload.tx_hash,
        amount=payload.amount,
        currency=payload.currency,
        external_escrow_id=payload.external_escrow_id,
        contract_address=payload.contract_address,
        escrow_balance=Decimal(str(payload.amount).strip()),
    )
    repo.save(agg)
    persist_domain_events(db, agg.pull_domain_events())

    event_row.status = DEPOSIT_EVENT_PROCESSED
    event_row.result_snapshot = agg.order.status
    event_row.processed_at = now
    db.commit()
    db.refresh(agg.order)
    if agg.escrow:
        db.refresh(agg.escrow)

    return DepositResult(
        order_id=payload.order_id,
        order_status=agg.order.status,
        escrow_funded=True,
        already_processed=False,
        rejected=False,
    )
