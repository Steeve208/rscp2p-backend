"""Create alerts table for persistent per-user alerts

Revision ID: 013_alerts
Revises: 012_launchpad_submissions
"""
from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa

revision: str = "013_alerts"
down_revision: Union[str, None] = "012_launchpad_submissions"
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    op.create_table(
        "alerts",
        sa.Column("id", sa.String(36), primary_key=True),
        sa.Column("user_id", sa.String(66), nullable=False, index=True),
        sa.Column("type", sa.String(32), nullable=False),
        sa.Column("title", sa.String(255), nullable=False),
        sa.Column("message", sa.Text(), nullable=False),
        sa.Column("severity", sa.String(16), nullable=False, server_default="medium"),
        sa.Column("data", sa.Text(), nullable=True),
        sa.Column("read", sa.Boolean(), nullable=False, server_default=sa.text("0")),
        sa.Column("created_at", sa.DateTime(), nullable=True),
    )
    op.create_index("ix_alerts_user_read", "alerts", ["user_id", "read"])
    op.create_index("ix_alerts_user_created", "alerts", ["user_id", "created_at"])


def downgrade() -> None:
    op.drop_index("ix_alerts_user_created", table_name="alerts")
    op.drop_index("ix_alerts_user_read", table_name="alerts")
    op.drop_table("alerts")
