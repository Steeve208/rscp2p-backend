"""MKT-001 a MKT-004: orders, escrows, disputes, users

Revision ID: 001_mkt
Revises:
Create Date: 2026-03-03

"""
from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa

revision: str = "001_mkt"
down_revision: Union[str, None] = None
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    # MKT-004 users (sin FK desde orders para evitar dependencia circular en migración)
    op.create_table(
        "users",
        sa.Column("id", sa.String(36), primary_key=True),
        sa.Column("wallet_address", sa.String(66), nullable=False),
        sa.Column("reputation_score", sa.Float(), nullable=False, server_default="0"),
        sa.Column("is_active", sa.Boolean(), nullable=False, server_default="1"),
        sa.Column("login_count", sa.Integer(), nullable=False, server_default="0"),
        sa.Column("last_login_at", sa.DateTime(), nullable=True),
        sa.Column("created_at", sa.DateTime(), nullable=False, server_default=sa.text("CURRENT_TIMESTAMP")),
        sa.Column("updated_at", sa.DateTime(), nullable=False, server_default=sa.text("CURRENT_TIMESTAMP")),
    )
    op.create_index("ix_users_wallet_address", "users", ["wallet_address"], unique=True)

    # MKT-001 orders
    op.create_table(
        "orders",
        sa.Column("id", sa.String(36), primary_key=True),
        sa.Column("seller_id", sa.String(64), nullable=False),
        sa.Column("buyer_id", sa.String(64), nullable=True),
        sa.Column("crypto_currency", sa.String(20), nullable=False),
        sa.Column("crypto_amount", sa.String(64), nullable=False),
        sa.Column("fiat_currency", sa.String(20), nullable=False),
        sa.Column("fiat_amount", sa.String(64), nullable=False),
        sa.Column("price_per_unit", sa.String(64), nullable=True),
        sa.Column("status", sa.String(32), nullable=False),
        sa.Column("payment_method", sa.String(120), nullable=True),
        sa.Column("terms", sa.Text(), nullable=True),
        sa.Column("expires_at", sa.DateTime(), nullable=True),
        sa.Column("escrow_id", sa.String(36), nullable=True),
        sa.Column("accepted_at", sa.DateTime(), nullable=True),
        sa.Column("completed_at", sa.DateTime(), nullable=True),
        sa.Column("cancelled_at", sa.DateTime(), nullable=True),
        sa.Column("cancelled_by", sa.String(16), nullable=True),
        sa.Column("disputed_at", sa.DateTime(), nullable=True),
        sa.Column("created_at", sa.DateTime(), nullable=False, server_default=sa.text("CURRENT_TIMESTAMP")),
        sa.Column("updated_at", sa.DateTime(), nullable=False, server_default=sa.text("CURRENT_TIMESTAMP")),
        sa.Column("seller_wallet", sa.String(66), nullable=True),
        sa.Column("seller_reputation", sa.String(16), nullable=True),
        sa.Column("buyer_wallet", sa.String(66), nullable=True),
        sa.Column("buyer_reputation", sa.String(16), nullable=True),
    )
    op.create_index("ix_orders_seller_id", "orders", ["seller_id"])
    op.create_index("ix_orders_buyer_id", "orders", ["buyer_id"])
    op.create_index("ix_orders_status", "orders", ["status"])
    op.create_index("ix_orders_seller_created", "orders", ["seller_id", "created_at"])
    op.create_index("ix_orders_buyer_created", "orders", ["buyer_id", "created_at"])
    op.create_index("ix_orders_market_status", "orders", ["crypto_currency", "fiat_currency", "status"])

    # MKT-002 escrows
    op.create_table(
        "escrows",
        sa.Column("id", sa.String(36), primary_key=True),
        sa.Column("order_id", sa.String(36), nullable=False),
        sa.Column("external_escrow_id", sa.String(128), nullable=False),
        sa.Column("contract_address", sa.String(66), nullable=False),
        sa.Column("crypto_amount", sa.String(64), nullable=False),
        sa.Column("crypto_currency", sa.String(20), nullable=False),
        sa.Column("status", sa.String(32), nullable=False, server_default="PENDING"),
        sa.Column("create_tx_hash", sa.String(66), nullable=True),
        sa.Column("release_tx_hash", sa.String(66), nullable=True),
        sa.Column("refund_tx_hash", sa.String(66), nullable=True),
        sa.Column("locked_at", sa.DateTime(), nullable=True),
        sa.Column("released_at", sa.DateTime(), nullable=True),
        sa.Column("refunded_at", sa.DateTime(), nullable=True),
        sa.Column("created_at", sa.DateTime(), nullable=False, server_default=sa.func.now()),
        sa.Column("updated_at", sa.DateTime(), nullable=False, server_default=sa.func.now()),
        sa.ForeignKeyConstraint(["order_id"], ["orders.id"]),
    )
    op.create_index("ix_escrows_order_id", "escrows", ["order_id"], unique=True)
    op.create_index("ix_escrows_external_escrow_id", "escrows", ["external_escrow_id"], unique=True)
    op.create_index("ix_escrows_status_updated", "escrows", ["status", "updated_at"])

    # MKT-003 disputes
    op.create_table(
        "disputes",
        sa.Column("id", sa.String(36), primary_key=True),
        sa.Column("order_id", sa.String(36), nullable=False),
        sa.Column("initiator_id", sa.String(64), nullable=False),
        sa.Column("respondent_id", sa.String(64), nullable=True),
        sa.Column("reason", sa.Text(), nullable=True),
        sa.Column("status", sa.String(32), nullable=False, server_default="OPEN"),
        sa.Column("resolution", sa.Text(), nullable=True),
        sa.Column("created_at", sa.DateTime(), nullable=False, server_default=sa.text("CURRENT_TIMESTAMP")),
        sa.Column("updated_at", sa.DateTime(), nullable=False, server_default=sa.text("CURRENT_TIMESTAMP")),
        sa.Column("resolved_at", sa.DateTime(), nullable=True),
        sa.ForeignKeyConstraint(["order_id"], ["orders.id"]),
    )
    op.create_index("ix_disputes_order_id", "disputes", ["order_id"])
    op.create_index("ix_disputes_status_created", "disputes", ["status", "created_at"])


def downgrade() -> None:
    op.drop_table("disputes")
    op.drop_table("escrows")
    op.drop_table("orders")
    op.drop_index("ix_users_wallet_address", table_name="users")
    op.drop_table("users")
