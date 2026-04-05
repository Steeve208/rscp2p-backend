"""Add response_status and response_hash to idempotency_keys for order mutation idempotency

Revision ID: 009_idempotency_order
Revises: 008_deposit_events
Create Date: 2026-03-09

Stored response (status + body) allows returning cached result on duplicate requests.
response_hash optional for verification of stored result.
"""
from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa

revision: str = "009_idempotency_order"
down_revision: Union[str, None] = "008_deposit_events"
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    op.add_column(
        "idempotency_keys",
        sa.Column("response_status", sa.Integer(), nullable=True),
    )
    op.add_column(
        "idempotency_keys",
        sa.Column("response_hash", sa.String(64), nullable=True),
    )
    op.add_column(
        "idempotency_keys",
        sa.Column("endpoint", sa.String(128), nullable=True),
    )


def downgrade() -> None:
    op.drop_column("idempotency_keys", "endpoint")
    op.drop_column("idempotency_keys", "response_hash")
    op.drop_column("idempotency_keys", "response_status")
