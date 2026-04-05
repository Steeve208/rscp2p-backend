"""Internal financial ledger: append-only ledger_entries for order escrow accounting

Revision ID: 007_ledger
Revises: 006_consistency
Create Date: 2026-03-09

Balances are derived from ledger entries; no balance columns stored.
Accounts: buyer_balance, escrow, seller_balance.
Double-entry: every transaction creates balanced entries (sum of amount = 0).
"""
from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa

revision: str = "007_ledger"
down_revision: Union[str, None] = "006_consistency"
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    op.create_table(
        "ledger_entries",
        sa.Column("id", sa.String(36), primary_key=True),
        sa.Column("order_id", sa.String(36), sa.ForeignKey("orders.id", ondelete="CASCADE"), nullable=False, index=True),
        sa.Column("account", sa.String(32), nullable=False, index=True),
        sa.Column("amount", sa.Numeric(36, 18), nullable=False),
        sa.Column("currency", sa.String(20), nullable=False, index=True),
        sa.Column("type", sa.String(32), nullable=False, index=True),
        sa.Column("reference_id", sa.String(128), nullable=True),
        sa.Column("created_at", sa.DateTime(), nullable=False, server_default=sa.func.now()),
    )
    op.create_index("ix_ledger_entries_order_account", "ledger_entries", ["order_id", "account"])
    op.create_index("ix_ledger_entries_order_created", "ledger_entries", ["order_id", "created_at"])


def downgrade() -> None:
    op.drop_index("ix_ledger_entries_order_created", table_name="ledger_entries")
    op.drop_index("ix_ledger_entries_order_account", table_name="ledger_entries")
    op.drop_table("ledger_entries")
