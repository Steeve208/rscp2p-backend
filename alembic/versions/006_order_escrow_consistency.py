"""Data migration: ensure Order and Escrow status consistency (no divergence)

Revision ID: 006_consistency
Revises: 005_outbox
Create Date: 2026-03-08

Aligns any legacy or inconsistent order/escrow rows so invariants hold:
- Order ESCROW_FUNDED/DISPUTED <=> Escrow FUNDED
- Order RELEASED <=> Escrow RELEASED
- Order CANCELLED (with escrow) <=> Escrow REFUNDED
"""
from typing import Sequence, Union

from alembic import op

revision: str = "006_consistency"
down_revision: Union[str, None] = "005_outbox"
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    # 1) Escrow FUNDED but order not ESCROW_FUNDED/DISPUTED -> set order to ESCROW_FUNDED (recoverable case)
    op.execute("""
        UPDATE orders SET status = 'ESCROW_FUNDED'
        WHERE id IN (SELECT order_id FROM escrows WHERE status = 'FUNDED')
        AND status NOT IN ('ESCROW_FUNDED', 'DISPUTED', 'RELEASED', 'CANCELLED')
    """)
    # 2) Order ESCROW_FUNDED but escrow not FUNDED -> set escrow to FUNDED (align to order)
    op.execute("""
        UPDATE escrows SET status = 'FUNDED'
        WHERE order_id IN (SELECT id FROM orders WHERE status = 'ESCROW_FUNDED')
        AND status = 'PENDING'
    """)
    # 3) Order RELEASED but escrow not RELEASED -> set escrow to RELEASED
    op.execute("""
        UPDATE escrows SET status = 'RELEASED'
        WHERE order_id IN (SELECT id FROM orders WHERE status = 'RELEASED')
        AND status IN ('PENDING', 'FUNDED')
    """)
    # 4) Order CANCELLED with escrow but escrow not REFUNDED -> set escrow to REFUNDED
    op.execute("""
        UPDATE escrows SET status = 'REFUNDED'
        WHERE order_id IN (SELECT id FROM orders WHERE status = 'CANCELLED')
        AND status IN ('PENDING', 'FUNDED')
    """)


def downgrade() -> None:
    # Data migration: no reversible mapping without backup; leave data as-is on downgrade
    pass
