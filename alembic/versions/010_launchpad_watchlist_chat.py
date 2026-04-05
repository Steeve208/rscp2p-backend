"""Launchpad: watchlist unique constraint, presale chat messages table

Revision ID: 010_launchpad
Revises: 009_idempotency_order
Create Date: 2026-03-15

- Add unique constraint (wallet_address, contract_address) on launchpad_watchlist
- Create launchpad_presale_chat_messages for presale chat persistence
"""
from typing import Sequence, Union

from alembic import op
from sqlalchemy import inspect
import sqlalchemy as sa

revision: str = "010_launchpad"
down_revision: Union[str, None] = "009_idempotency_order"
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    bind = op.get_bind()
    insp = inspect(bind)

    if not insp.has_table("launchpad_presale_chat_messages"):
        op.create_table(
            "launchpad_presale_chat_messages",
            sa.Column("id", sa.String(36), primary_key=True),
            sa.Column("presale_id", sa.String(36), sa.ForeignKey("launchpad_presales.id"), nullable=False, index=True),
            sa.Column("user_id", sa.String(66), nullable=False, index=True),
            sa.Column("message", sa.Text(), nullable=False),
            sa.Column("created_at", sa.DateTime(), nullable=True, server_default=sa.text("CURRENT_TIMESTAMP")),
        )

    if insp.has_table("launchpad_watchlist"):
        try:
            op.create_unique_constraint(
                "uq_watchlist_user_token",
                "launchpad_watchlist",
                ["wallet_address", "contract_address"],
            )
        except Exception:
            pass  # Constraint may already exist


def downgrade() -> None:
    op.drop_constraint("uq_watchlist_user_token", "launchpad_watchlist", type_="unique")
    op.drop_table("launchpad_presale_chat_messages")
