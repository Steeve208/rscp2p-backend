"""Rename order and escrow status values (production consistency)

Revision ID: 004_status_rename
Revises: 003_idempotency
Create Date: 2026-03-08

Maps legacy status strings to production names so Order/Escrow stay consistent.
"""
from typing import Sequence, Union

from alembic import op

revision: str = "004_status_rename"
down_revision: Union[str, None] = "003_idempotency"
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    # Order status: legacy -> production
    op.execute("""
        UPDATE orders SET status = 'AWAITING_PAYMENT' WHERE status = 'AWAITING_FUNDS'
    """)
    op.execute("""
        UPDATE orders SET status = 'ESCROW_FUNDED' WHERE status = 'ONCHAIN_LOCKED'
    """)
    op.execute("""
        UPDATE orders SET status = 'RELEASED' WHERE status = 'COMPLETED'
    """)
    op.execute("""
        UPDATE orders SET status = 'CANCELLED' WHERE status = 'REFUNDED'
    """)
    # Escrow status
    op.execute("""
        UPDATE escrows SET status = 'FUNDED' WHERE status = 'LOCKED'
    """)


def downgrade() -> None:
    op.execute("""
        UPDATE orders SET status = 'AWAITING_FUNDS' WHERE status = 'AWAITING_PAYMENT'
    """)
    op.execute("""
        UPDATE orders SET status = 'ONCHAIN_LOCKED' WHERE status = 'ESCROW_FUNDED'
    """)
    op.execute("""
        UPDATE orders SET status = 'COMPLETED' WHERE status = 'RELEASED'
    """)
    op.execute("""
        UPDATE orders SET status = 'REFUNDED' WHERE status = 'CANCELLED'
    """)
    op.execute("""
        UPDATE escrows SET status = 'LOCKED' WHERE status = 'FUNDED'
    """)
