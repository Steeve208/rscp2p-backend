"""
Internal financial ledger for order escrow.
Balances are never stored; they are derived from immutable ledger entries.
Double-entry: every transaction creates balanced entries (sum of amount = 0 per transaction).
"""

from dataclasses import dataclass
from datetime import datetime
from decimal import Decimal
from typing import Optional

# Ledger accounts (per order)
ACCOUNT_BUYER_BALANCE = "buyer_balance"
ACCOUNT_ESCROW = "escrow"
ACCOUNT_SELLER_BALANCE = "seller_balance"

LEDGER_ACCOUNTS = frozenset({ACCOUNT_BUYER_BALANCE, ACCOUNT_ESCROW, ACCOUNT_SELLER_BALANCE})

# Entry types (transaction type)
ENTRY_TYPE_DEPOSIT = "DEPOSIT"
ENTRY_TYPE_RELEASE = "RELEASE"
ENTRY_TYPE_REFUND = "REFUND"


@dataclass(frozen=True)
class LedgerEntry:
    """
    Immutable ledger entry. Append-only; never updated or deleted.
    amount: signed (positive = credit, negative = debit). Balance = sum(amount) per account.
    """

    id: str
    order_id: str
    account: str
    amount: Decimal
    currency: str
    type: str
    reference_id: Optional[str]
    created_at: datetime

    def __post_init__(self) -> None:
        if self.account not in LEDGER_ACCOUNTS:
            raise ValueError(f"Invalid ledger account: {self.account}")
