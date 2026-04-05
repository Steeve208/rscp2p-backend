"""Production constraints and auth nonces

Revision ID: 011_production
Revises: 010_launchpad
Create Date: 2026-03-16

- UNIQUE (presale_id, tx_hash) on launchpad_contributions
- UNIQUE (contract_address, wallet_address) on launchpad_sentiment_votes
- UNIQUE (order_id, tx_hash) on deposit_events
- auth_nonces table for multi-instance login
"""
from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa

revision: str = "011_production"
down_revision: Union[str, None] = "010_launchpad"
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    bind = op.get_bind()
    insp = sa.inspect(bind)

    if insp.has_table("launchpad_contributions"):
        try:
            op.create_unique_constraint(
                "uq_contributions_presale_tx",
                "launchpad_contributions",
                ["presale_id", "tx_hash"],
            )
        except Exception:
            pass

    if insp.has_table("launchpad_sentiment_votes"):
        try:
            op.create_unique_constraint(
                "uq_sentiment_vote_token_wallet",
                "launchpad_sentiment_votes",
                ["contract_address", "wallet_address"],
            )
        except Exception:
            pass

    if insp.has_table("deposit_events"):
        try:
            op.create_unique_constraint(
                "uq_deposit_events_order_tx",
                "deposit_events",
                ["order_id", "tx_hash"],
            )
        except Exception:
            pass

    if not insp.has_table("auth_nonces"):
        op.create_table(
            "auth_nonces",
            sa.Column("wallet_address", sa.String(66), primary_key=True),
            sa.Column("nonce", sa.String(64), nullable=False),
            sa.Column("expires_at", sa.DateTime(), nullable=False),
            sa.Column("created_at", sa.DateTime(), nullable=False, server_default=sa.func.now()),
        )
        op.create_index("ix_auth_nonces_expires_at", "auth_nonces", ["expires_at"])


def downgrade() -> None:
    op.drop_index("ix_auth_nonces_expires_at", table_name="auth_nonces")
    op.drop_table("auth_nonces")

    try:
        op.drop_constraint("uq_deposit_events_order_tx", "deposit_events", type_="unique")
    except Exception:
        pass
    try:
        op.drop_constraint("uq_sentiment_vote_token_wallet", "launchpad_sentiment_votes", type_="unique")
    except Exception:
        pass
    try:
        op.drop_constraint("uq_contributions_presale_tx", "launchpad_contributions", type_="unique")
    except Exception:
        pass
