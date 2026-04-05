"""
Unit tests for internal financial ledger.
- Ledger entries are immutable; balances derived by summing entries.
- Double-entry: every transaction must balance (sum amount = 0 per currency).
"""

import uuid
from decimal import Decimal

import pytest
from sqlalchemy import create_engine, select
from sqlalchemy.orm import sessionmaker, Session

from app.db import Base
from app.domain.ledger import (
    ACCOUNT_BUYER_BALANCE,
    ACCOUNT_ESCROW,
    ACCOUNT_SELLER_BALANCE,
    LedgerEntry,
)
from app.models import LedgerEntryModel  # noqa: F401 - register for create_all
from app.models.marketplace import OrderModel
from app.services.ledger_service import (
    LedgerError,
    create_balanced_entries,
    entries_for_deposit,
    entries_for_release,
    entries_for_refund,
    get_balance,
)


@pytest.fixture
def db():
    engine = create_engine("sqlite:///:memory:", connect_args={"check_same_thread": False})
    Base.metadata.create_all(bind=engine)
    SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)
    session = SessionLocal()
    yield session
    session.close()


@pytest.fixture
def order_id():
    return f"ord-{uuid.uuid4().hex[:12]}"


def test_ledger_entry_domain_valid_account():
    from datetime import datetime, timezone
    LedgerEntry(
        id="e1",
        order_id="o1",
        account=ACCOUNT_ESCROW,
        amount=Decimal("100"),
        currency="USDT",
        type="DEPOSIT",
        reference_id="tx1",
        created_at=datetime.now(timezone.utc),
    )


def test_ledger_entry_domain_invalid_account_raises():
    from datetime import datetime, timezone
    with pytest.raises(ValueError, match="Invalid ledger account"):
        LedgerEntry(
            id="e1",
            order_id="o1",
            account="invalid_account",
            amount=Decimal("100"),
            currency="USDT",
            type="DEPOSIT",
            reference_id=None,
            created_at=datetime.now(timezone.utc),
        )


def test_create_balanced_entries_rejects_unbalanced(db: Session, order_id: str):
    """Unbalanced transaction (sum != 0) must raise LedgerError."""
    entries = [
        (ACCOUNT_BUYER_BALANCE, Decimal("-100"), "USDT", "DEPOSIT", "tx1"),
        (ACCOUNT_ESCROW, Decimal("50"), "USDT", "DEPOSIT", "tx1"),  # sum = -50
    ]
    with pytest.raises(LedgerError, match="Unbalanced"):
        create_balanced_entries(db, order_id, entries)


def test_create_balanced_entries_accepts_balanced(db: Session, order_id: str):
    """Balanced double-entry is persisted."""
    entries = [
        (ACCOUNT_BUYER_BALANCE, Decimal("-100"), "USDT", "DEPOSIT", "tx1"),
        (ACCOUNT_ESCROW, Decimal("100"), "USDT", "DEPOSIT", "tx1"),
    ]
    created = create_balanced_entries(db, order_id, entries)
    db.commit()
    assert len(created) == 2
    rows = list(db.scalars(select(LedgerEntryModel).where(LedgerEntryModel.order_id == order_id)))
    assert len(rows) == 2
    accounts = {r.account for r in rows}
    assert accounts == {ACCOUNT_BUYER_BALANCE, ACCOUNT_ESCROW}
    total = sum(r.amount for r in rows)
    assert total == 0


def test_get_balance_sums_entries(db: Session, order_id: str):
    """Balance for an account is the sum of ledger entry amounts."""
    entries = [
        (ACCOUNT_ESCROW, Decimal("100"), "USDT", "DEPOSIT", "tx1"),
        (ACCOUNT_ESCROW, Decimal("-40"), "USDT", "RELEASE", "tx2"),
    ]
    create_balanced_entries(db, order_id, [
        (ACCOUNT_BUYER_BALANCE, Decimal("-100"), "USDT", "DEPOSIT", "tx1"),
        (ACCOUNT_ESCROW, Decimal("100"), "USDT", "DEPOSIT", "tx1"),
    ])
    create_balanced_entries(db, order_id, [
        (ACCOUNT_ESCROW, Decimal("-40"), "USDT", "RELEASE", "tx2"),
        (ACCOUNT_SELLER_BALANCE, Decimal("40"), "USDT", "RELEASE", "tx2"),
    ])
    db.commit()

    assert get_balance(db, order_id, ACCOUNT_BUYER_BALANCE, "USDT") == Decimal("-100")
    assert get_balance(db, order_id, ACCOUNT_ESCROW, "USDT") == Decimal("60")
    assert get_balance(db, order_id, ACCOUNT_SELLER_BALANCE, "USDT") == Decimal("40")


def test_get_balance_no_entries_returns_zero(db: Session, order_id: str):
    assert get_balance(db, order_id, ACCOUNT_ESCROW, "USDT") == Decimal("0")


def test_entries_for_deposit_double_entry():
    """Deposit: buyer_balance -amount, escrow +amount; sum = 0."""
    entries = entries_for_deposit("o1", "100", "USDT", "tx1")
    assert len(entries) == 2
    by_account = {e[0]: e[1] for e in entries}
    assert by_account[ACCOUNT_BUYER_BALANCE] == Decimal("-100")
    assert by_account[ACCOUNT_ESCROW] == Decimal("100")
    assert sum(e[1] for e in entries) == 0


def test_entries_for_release_double_entry():
    """Release: escrow -amount, seller_balance +amount; sum = 0."""
    entries = entries_for_release("o1", Decimal("100"), "USDT", "tx2")
    assert len(entries) == 2
    by_account = {e[0]: e[1] for e in entries}
    assert by_account[ACCOUNT_ESCROW] == Decimal("-100")
    assert by_account[ACCOUNT_SELLER_BALANCE] == Decimal("100")
    assert sum(e[1] for e in entries) == 0


def test_entries_for_refund_double_entry():
    """Refund: escrow -amount, buyer_balance +amount; sum = 0."""
    entries = entries_for_refund("o1", "100", "USDT", None)
    assert len(entries) == 2
    by_account = {e[0]: e[1] for e in entries}
    assert by_account[ACCOUNT_ESCROW] == Decimal("-100")
    assert by_account[ACCOUNT_BUYER_BALANCE] == Decimal("100")
    assert sum(e[1] for e in entries) == 0


def test_create_balanced_entries_empty_raises(db: Session, order_id: str):
    with pytest.raises(LedgerError, match="At least one entry"):
        create_balanced_entries(db, order_id, [])


def test_ledger_consistency_deposit_then_release(db: Session, order_id: str):
    """After deposit then release: escrow balance 0, seller_balance = amount."""
    deposit_entries = entries_for_deposit(order_id, "100", "USDT", "tx1")
    create_balanced_entries(db, order_id, deposit_entries)
    release_entries = entries_for_release(order_id, "100", "USDT", "tx2")
    create_balanced_entries(db, order_id, release_entries)
    db.commit()

    assert get_balance(db, order_id, ACCOUNT_BUYER_BALANCE, "USDT") == Decimal("-100")
    assert get_balance(db, order_id, ACCOUNT_ESCROW, "USDT") == Decimal("0")
    assert get_balance(db, order_id, ACCOUNT_SELLER_BALANCE, "USDT") == Decimal("100")


def test_ledger_consistency_deposit_then_refund(db: Session, order_id: str):
    """After deposit then refund: escrow balance 0, buyer_balance restored to 0."""
    deposit_entries = entries_for_deposit(order_id, "100", "USDT", "tx1")
    create_balanced_entries(db, order_id, deposit_entries)
    refund_entries = entries_for_refund(order_id, "100", "USDT", "tx3")
    create_balanced_entries(db, order_id, refund_entries)
    db.commit()

    assert get_balance(db, order_id, ACCOUNT_BUYER_BALANCE, "USDT") == Decimal("0")
    assert get_balance(db, order_id, ACCOUNT_ESCROW, "USDT") == Decimal("0")
    assert get_balance(db, order_id, ACCOUNT_SELLER_BALANCE, "USDT") == Decimal("0")
