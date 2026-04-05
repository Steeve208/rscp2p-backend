"""
Deterministic deposit processing pipeline tests.

Covers: duplicate deposit events, concurrent deposits, partial deposits, overpayment prevention.
Concurrency stress: 20 workers processing same deposit → exactly once, ledger invariants hold.
"""

import tempfile
import uuid
from concurrent.futures import ThreadPoolExecutor, as_completed
from decimal import Decimal
from pathlib import Path

import pytest
from fastapi import HTTPException
from sqlalchemy import create_engine, select
from sqlalchemy.orm import sessionmaker, Session

from app.db import Base
from app.domain.ledger import ACCOUNT_ESCROW
from app.models import DepositEventModel, LedgerEntryModel  # noqa: F401 - register for create_all
from app.models.marketplace import EscrowModel, OrderModel
from app.services.deposit_processor import DepositEventPayload, process_deposit_event
from app.services.ledger_service import get_balance


@pytest.fixture
def db():
    engine = create_engine("sqlite:///:memory:", connect_args={"check_same_thread": False})
    Base.metadata.create_all(bind=engine)
    SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)
    session = SessionLocal()
    yield session
    session.close()


@pytest.fixture
def db_engine():
    """
    Shared engine for concurrent workers; each worker gets its own session.
    Use a file-backed SQLite so all threads share the same database (in-memory
    would give each connection its own DB and break the test).
    """
    tmp = tempfile.NamedTemporaryFile(suffix=".db", delete=False)
    tmp.close()
    uri = f"sqlite:///{tmp.name}"
    engine = create_engine(uri, connect_args={"check_same_thread": False})
    Base.metadata.create_all(bind=engine)
    try:
        yield engine
    finally:
        engine.dispose()
        Path(tmp.name).unlink(missing_ok=True)


def _make_order(
    db: Session,
    order_id: str,
    status: str = "AWAITING_PAYMENT",
    buyer_id: str = "buyer-1",
    crypto_amount: str = "100",
    crypto_currency: str = "USDT",
) -> OrderModel:
    o = OrderModel(
        id=order_id,
        seller_id="seller-1",
        buyer_id=buyer_id,
        seller_wallet="0xseller",
        buyer_wallet="0xbuyer",
        crypto_currency=crypto_currency,
        crypto_amount=crypto_amount,
        fiat_currency="USD",
        fiat_amount="100",
        price_per_unit="1",
        status=status,
        payment_method="Wallet",
    )
    db.add(o)
    db.commit()
    db.refresh(o)
    return o


def test_duplicate_deposit_events_same_key_return_cached(db: Session):
    """Same idempotency key twice: first processes, second returns cached result (exactly once)."""
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, order_id)
    payload = DepositEventPayload(
        order_id=order_id,
        tx_hash="0xtx-dup",
        amount="100",
        currency="USDT",
        external_escrow_id="ext-1",
        contract_address="0xc",
        idempotency_key="dup-key-1",
    )
    r1 = process_deposit_event(db, payload)
    assert r1.order_status == "ESCROW_FUNDED"
    assert r1.escrow_funded is True
    assert r1.already_processed is False

    r2 = process_deposit_event(db, payload)
    assert r2.order_status == "ESCROW_FUNDED"
    assert r2.escrow_funded is True
    assert r2.already_processed is True

    events = list(db.scalars(select(DepositEventModel).where(DepositEventModel.order_id == order_id)))
    assert len(events) == 1
    assert events[0].status == "PROCESSED"


def test_concurrent_deposits_same_key_only_one_processes(db: Session):
    """Same idempotency key processed 5 times: first applies (row lock + idempotency), rest return cached."""
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, order_id)
    payload = DepositEventPayload(
        order_id=order_id,
        tx_hash="0xtx-concurrent",
        amount="100",
        currency="USDT",
        external_escrow_id="ext-1",
        contract_address="0xc",
        idempotency_key="concurrent-key-xyz",
    )
    results = [process_deposit_event(db, payload) for _ in range(5)]
    applied = [r for r in results if not r.already_processed]
    already = [r for r in results if r.already_processed]
    assert len(applied) == 1
    assert len(already) == 4
    assert applied[0].order_status == "ESCROW_FUNDED"
    events = list(db.scalars(select(DepositEventModel).where(DepositEventModel.idempotency_key == "concurrent-key-xyz")))
    assert len(events) == 1
    assert events[0].status == "PROCESSED"


def test_partial_deposit_rejected(db: Session):
    """Partial deposit (amount < order amount) is rejected; deposit_event status = REJECTED."""
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, order_id, crypto_amount="100")
    payload = DepositEventPayload(
        order_id=order_id,
        tx_hash="0xtx-partial",
        amount="50",
        currency="USDT",
        external_escrow_id="ext-1",
        contract_address="0xc",
        idempotency_key="partial-key-1",
    )
    with pytest.raises(HTTPException) as exc:
        process_deposit_event(db, payload)
    assert exc.value.status_code == 409
    assert "Partial" in (exc.value.detail or "") or "amount" in (exc.value.detail or "").lower()
    events = list(db.scalars(select(DepositEventModel).where(DepositEventModel.order_id == order_id)))
    assert len(events) == 1
    assert events[0].status == "REJECTED"
    assert "Partial" in (events[0].rejection_reason or "") or "partial" in (events[0].rejection_reason or "").lower()


def test_overpayment_prevention(db: Session):
    """Overpayment (amount > order amount) is rejected; deposit_event status = REJECTED."""
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, order_id, crypto_amount="100")
    payload = DepositEventPayload(
        order_id=order_id,
        tx_hash="0xtx-over",
        amount="150",
        currency="USDT",
        external_escrow_id="ext-1",
        contract_address="0xc",
        idempotency_key="overpay-key-1",
    )
    with pytest.raises(HTTPException) as exc:
        process_deposit_event(db, payload)
    assert exc.value.status_code == 409
    assert "Overpayment" in (exc.value.detail or "") or "amount" in (exc.value.detail or "").lower()
    events = list(db.scalars(select(DepositEventModel).where(DepositEventModel.order_id == order_id)))
    assert len(events) == 1
    assert events[0].status == "REJECTED"
    assert "Overpayment" in (events[0].rejection_reason or "") or "overpayment" in (events[0].rejection_reason or "").lower()


def test_transaction_boundaries_deposit_event_processed_after_commit(db: Session):
    """After successful process, deposit_events has one row with status PROCESSED and result_snapshot."""
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, order_id)
    payload = DepositEventPayload(
        order_id=order_id,
        tx_hash="0xtx-ok",
        amount="100",
        currency="USDT",
        external_escrow_id="ext-1",
        contract_address="0xc",
        idempotency_key="tx-boundary-1",
    )
    r = process_deposit_event(db, payload)
    assert r.order_status == "ESCROW_FUNDED"
    event = db.scalar(select(DepositEventModel).where(DepositEventModel.idempotency_key == "tx-boundary-1"))
    assert event is not None
    assert event.status == "PROCESSED"
    assert event.result_snapshot == "ESCROW_FUNDED"
    assert event.processed_at is not None


def test_second_deposit_same_order_different_tx_rejected(db: Session):
    """
    Second deposit for the same order (different tx_hash) must be rejected:
    order is already ESCROW_FUNDED, so apply_deposit is not allowed; escrow ledger balance must not change.
    """
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    _make_order(db, order_id, crypto_amount="100")
    payload1 = DepositEventPayload(
        order_id=order_id,
        tx_hash="0xtx-first",
        amount="100",
        currency="USDT",
        external_escrow_id="ext-1",
        contract_address="0xc",
        idempotency_key="first-deposit-key",
    )
    r1 = process_deposit_event(db, payload1)
    assert r1.order_status == "ESCROW_FUNDED"
    assert r1.already_processed is False
    escrow_balance_after_first = get_balance(db, order_id, ACCOUNT_ESCROW, "USDT")
    assert escrow_balance_after_first == Decimal("100")

    payload2 = DepositEventPayload(
        order_id=order_id,
        tx_hash="0xtx-second",
        amount="100",
        currency="USDT",
        external_escrow_id="ext-2",
        contract_address="0xc",
        idempotency_key="second-deposit-key",
    )
    with pytest.raises(HTTPException) as exc:
        process_deposit_event(db, payload2)
    assert exc.value.status_code == 409
    assert "AWAITING_PAYMENT" in (exc.value.detail or "") or "Deposit" in (exc.value.detail or "")
    escrow_balance_after_second = get_balance(db, order_id, ACCOUNT_ESCROW, "USDT")
    assert escrow_balance_after_second == Decimal("100"), "Escrow balance must not double-credit"


def test_order_not_found_deposit_event_rejected(db: Session):
    """Order not found: deposit_event is stored and marked REJECTED; 404 raised."""
    order_id = f"ord-{uuid.uuid4().hex[:12]}"
    payload = DepositEventPayload(
        order_id=order_id,
        tx_hash="0xtx-404",
        amount="100",
        currency="USDT",
        external_escrow_id="ext-1",
        contract_address="0xc",
        idempotency_key="404-key-1",
    )
    with pytest.raises(HTTPException) as exc:
        process_deposit_event(db, payload)
    assert exc.value.status_code == 404
    event = db.scalar(select(DepositEventModel).where(DepositEventModel.idempotency_key == "404-key-1"))
    assert event is not None
    assert event.status == "REJECTED"
    assert "not found" in (event.rejection_reason or "").lower()


# ----- Concurrency stress: 20 workers, same order -----

def test_concurrency_stress_20_workers_same_order_one_deposit_applied(
    db_engine,
):
    """
    Concurrency stress: spawn 20 workers all processing the same deposit for the same order.

    Expected behavior:
    - Idempotency (deposit_events.idempotency_key UNIQUE): only one worker wins the INSERT.
    - The winner runs: lock order → ledger entries → escrow update → domain event → commit.
    - The other 19 workers get IntegrityError on INSERT deposit_events, then return cached result
      (already_processed=True) without applying the deposit again.

    Invariants verified after all workers finish:
    1. Order state is ESCROW_FUNDED.
    2. Escrow ledger balance equals order amount (exactly one deposit applied).
    3. No duplicate ledger entries: exactly 2 ledger rows for this order (one deposit = 2 entries:
       buyer_balance -amount, escrow +amount).
    4. Exactly one deposit_event row with status PROCESSED.
    """
    order_id = f"ord-stress-{uuid.uuid4().hex[:12]}"
    order_amount = "100"
    currency = "USDT"
    idempotency_key = f"stress-key-{uuid.uuid4().hex[:16]}"
    num_workers = 20

    # Bootstrap: create order in a single session
    SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=db_engine)
    with SessionLocal() as bootstrap:
        _make_order(bootstrap, order_id, crypto_amount=order_amount, crypto_currency=currency)
        bootstrap.commit()

    payload = DepositEventPayload(
        order_id=order_id,
        tx_hash="0xtx-stress-concurrent",
        amount=order_amount,
        currency=currency,
        external_escrow_id="ext-stress",
        contract_address="0xc",
        idempotency_key=idempotency_key,
    )

    results = []
    errors = []

    def worker():
        session = SessionLocal()
        try:
            r = process_deposit_event(session, payload)
            return ("ok", r)
        except Exception as e:
            return ("error", e)
        finally:
            session.close()

    with ThreadPoolExecutor(max_workers=num_workers) as executor:
        futures = [executor.submit(worker) for _ in range(num_workers)]
        for f in as_completed(futures):
            status, value = f.result()
            if status == "ok":
                results.append(value)
            else:
                errors.append(value)

    # No worker should raise (duplicates get cached result, not an exception)
    assert len(errors) == 0, f"Workers raised: {errors}"
    assert len(results) == num_workers

    applied = [r for r in results if not r.already_processed]
    cached = [r for r in results if r.already_processed]
    assert len(applied) == 1, "Exactly one worker must apply the deposit (exactly-once)"
    assert len(cached) == num_workers - 1
    assert applied[0].order_status == "ESCROW_FUNDED"
    assert applied[0].escrow_funded is True

    # Verify invariants in a fresh session (see committed state)
    with SessionLocal() as db:
        order = db.get(OrderModel, order_id)
        assert order is not None
        assert order.status == "ESCROW_FUNDED", "Order state must be ESCROW_FUNDED"

        # Escrow ledger balance equals order amount
        escrow_balance = get_balance(db, order_id, ACCOUNT_ESCROW, currency)
        assert escrow_balance == Decimal(order_amount), (
            f"Escrow ledger balance must equal order amount: got {escrow_balance}, expected {order_amount}"
        )

        # No duplicate ledger entries: one deposit = 2 entries (buyer debit, escrow credit)
        ledger_rows = list(db.scalars(select(LedgerEntryModel).where(LedgerEntryModel.order_id == order_id)))
        assert len(ledger_rows) == 2, (
            f"Expected exactly 2 ledger entries (one deposit), got {len(ledger_rows)}; duplicate entries would break invariant"
        )
        accounts = {r.account for r in ledger_rows}
        assert ACCOUNT_ESCROW in accounts
        assert "buyer_balance" in accounts

        # Exactly one deposit_event processed
        deposit_events = list(
            db.scalars(select(DepositEventModel).where(DepositEventModel.order_id == order_id))
        )
        assert len(deposit_events) == 1
        assert deposit_events[0].status == "PROCESSED"
        assert deposit_events[0].result_snapshot == "ESCROW_FUNDED"
