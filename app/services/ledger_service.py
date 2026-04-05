"""
Internal financial ledger service.
Append-only ledger entries; balances derived by summing entries.
Double-entry: every transaction must create balanced entries (sum amount = 0).
"""

from decimal import Decimal
from uuid import uuid4

from sqlalchemy import select
from sqlalchemy.orm import Session

from app.domain.ledger import (
    ACCOUNT_BUYER_BALANCE,
    ACCOUNT_ESCROW,
    ACCOUNT_SELLER_BALANCE,
    ENTRY_TYPE_DEPOSIT,
    ENTRY_TYPE_REFUND,
    ENTRY_TYPE_RELEASE,
    LedgerEntry,
)
from app.models.marketplace import LedgerEntryModel


class LedgerError(Exception):
    """Raised when ledger invariants are violated (e.g. unbalanced transaction)."""

    pass


def _decimal_amount(amount: str | Decimal) -> Decimal:
    if isinstance(amount, Decimal):
        return amount
    return Decimal(str(amount).strip())


def create_balanced_entries(
    db: Session,
    order_id: str,
    entries: list[tuple[str, Decimal, str, str, str | None]],
    created_at=None,
) -> list[LedgerEntryModel]:
    """
    Persist a set of ledger entries that must balance (sum of amount = 0 per currency).
    Each tuple: (account, amount, currency, type, reference_id).
    Returns the created LedgerEntryModel instances (already added to session).
    """
    if not entries:
        raise LedgerError("At least one entry required")

    # Validate balance per currency
    by_currency: dict[str, Decimal] = {}
    for _acc, amount, currency, _typ, _ref in entries:
        by_currency[currency] = by_currency.get(currency, Decimal("0")) + amount
    for currency, total in by_currency.items():
        if total != 0:
            raise LedgerError(f"Unbalanced transaction for currency {currency}: sum={total}")

    now = created_at
    if now is None:
        from datetime import datetime, timezone
        now = datetime.now(timezone.utc)

    models = []
    for account, amount, currency, entry_type, reference_id in entries:
        entry_id = str(uuid4())
        m = LedgerEntryModel(
            id=entry_id,
            order_id=order_id,
            account=account,
            amount=amount,
            currency=currency,
            type=entry_type,
            reference_id=reference_id,
            created_at=now,
        )
        db.add(m)
        models.append(m)
    return models


def get_balance(db: Session, order_id: str, account: str, currency: str) -> Decimal:
    """Compute balance for an order's account by summing immutable ledger entries."""
    from sqlalchemy import func
    result = db.scalar(
        select(func.coalesce(func.sum(LedgerEntryModel.amount), 0)).where(
            LedgerEntryModel.order_id == order_id,
            LedgerEntryModel.account == account,
            LedgerEntryModel.currency == currency,
        )
    )
    if result is None:
        return Decimal("0")
    return Decimal(str(result))


# --- Convenience builders for order lifecycle (double-entry) ---


def entries_for_deposit(order_id: str, amount: str | Decimal, currency: str, reference_id: str | None) -> list[tuple[str, Decimal, str, str, str | None]]:
    """
    Buyer funds escrow: debit buyer_balance, credit escrow.
    amount is the positive amount being deposited.
    """
    amt = _decimal_amount(amount)
    return [
        (ACCOUNT_BUYER_BALANCE, -amt, currency, ENTRY_TYPE_DEPOSIT, reference_id),
        (ACCOUNT_ESCROW, amt, currency, ENTRY_TYPE_DEPOSIT, reference_id),
    ]


def entries_for_release(order_id: str, amount: str | Decimal, currency: str, reference_id: str | None) -> list[tuple[str, Decimal, str, str, str | None]]:
    """
    Release to seller: debit escrow, credit seller_balance.
    """
    amt = _decimal_amount(amount)
    return [
        (ACCOUNT_ESCROW, -amt, currency, ENTRY_TYPE_RELEASE, reference_id),
        (ACCOUNT_SELLER_BALANCE, amt, currency, ENTRY_TYPE_RELEASE, reference_id),
    ]


def entries_for_refund(order_id: str, amount: str | Decimal, currency: str, reference_id: str | None) -> list[tuple[str, Decimal, str, str, str | None]]:
    """
    Refund to buyer: debit escrow, credit buyer_balance.
    """
    amt = _decimal_amount(amount)
    return [
        (ACCOUNT_ESCROW, -amt, currency, ENTRY_TYPE_REFUND, reference_id),
        (ACCOUNT_BUYER_BALANCE, amt, currency, ENTRY_TYPE_REFUND, reference_id),
    ]
