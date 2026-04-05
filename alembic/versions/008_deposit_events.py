"""Deposit events table: audit and idempotency for blockchain deposit pipeline

Revision ID: 008_deposit_events
Revises: 007_ledger
Create Date: 2026-03-09

Each deposit event is stored once (unique idempotency_key). Status: PENDING -> PROCESSED | REJECTED.
Single transaction: insert event -> lock order -> ledger -> escrow -> domain event -> update status.
"""
from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa

revision: str = "008_deposit_events"
down_revision: Union[str, None] = "007_ledger"
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    op.create_table(
        "deposit_events",
        sa.Column("id", sa.String(36), primary_key=True),
        sa.Column("idempotency_key", sa.String(255), nullable=False, unique=True, index=True),
        sa.Column("order_id", sa.String(36), nullable=False, index=True),
        sa.Column("tx_hash", sa.String(66), nullable=False),
        sa.Column("amount", sa.String(64), nullable=False),
        sa.Column("currency", sa.String(20), nullable=False),
        sa.Column("external_escrow_id", sa.String(128), nullable=False),
        sa.Column("contract_address", sa.String(66), nullable=False),
        sa.Column("status", sa.String(32), nullable=False, server_default="PENDING", index=True),
        sa.Column("result_snapshot", sa.Text(), nullable=True),
        sa.Column("rejection_reason", sa.String(512), nullable=True),
        sa.Column("processed_at", sa.DateTime(), nullable=True),
        sa.Column("created_at", sa.DateTime(), nullable=False, server_default=sa.func.now()),
    )
    op.create_index("ix_deposit_events_order_id", "deposit_events", ["order_id"])
    op.create_index("ix_deposit_events_status_created", "deposit_events", ["status", "created_at"])


def downgrade() -> None:
    op.drop_index("ix_deposit_events_status_created", table_name="deposit_events")
    op.drop_index("ix_deposit_events_order_id", table_name="deposit_events")
    op.drop_table("deposit_events")
