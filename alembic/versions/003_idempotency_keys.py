"""Idempotency keys table for deposit and other events

Revision ID: 003_idempotency
Revises: 002_domain_events
Create Date: 2026-03-08

"""
from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa

revision: str = "003_idempotency"
down_revision: Union[str, None] = "002_domain_events"
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    op.create_table(
        "idempotency_keys",
        sa.Column("id", sa.String(36), primary_key=True),
        sa.Column("idempotency_key", sa.String(255), nullable=False),
        sa.Column("order_id", sa.String(36), nullable=False),
        sa.Column("event_type", sa.String(64), nullable=False),
        sa.Column("result_snapshot", sa.Text(), nullable=True),
        sa.Column("created_at", sa.DateTime(), nullable=False),
    )
    op.create_index("ix_idempotency_keys_idempotency_key", "idempotency_keys", ["idempotency_key"], unique=True)
    op.create_index("ix_idempotency_keys_order_id", "idempotency_keys", ["order_id"])


def downgrade() -> None:
    op.drop_index("ix_idempotency_keys_order_id", table_name="idempotency_keys")
    op.drop_index("ix_idempotency_keys_idempotency_key", table_name="idempotency_keys")
    op.drop_table("idempotency_keys")
