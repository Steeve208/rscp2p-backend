"""Domain events table (event log for Order aggregate)

Revision ID: 002_domain_events
Revises: 001_mkt
Create Date: 2026-03-08

"""
from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa

revision: str = "002_domain_events"
down_revision: Union[str, None] = "001_mkt"
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    op.create_table(
        "domain_events",
        sa.Column("id", sa.String(36), primary_key=True),
        sa.Column("order_id", sa.String(36), nullable=False),
        sa.Column("event_type", sa.String(64), nullable=False),
        sa.Column("payload", sa.Text(), nullable=False),
        sa.Column("occurred_at", sa.DateTime(), nullable=False),
    )
    op.create_index("ix_domain_events_order_id", "domain_events", ["order_id"])
    op.create_index("ix_domain_events_event_type", "domain_events", ["event_type"])
    op.create_index("ix_domain_events_occurred_at", "domain_events", ["occurred_at"])


def downgrade() -> None:
    op.drop_index("ix_domain_events_occurred_at", table_name="domain_events")
    op.drop_index("ix_domain_events_event_type", table_name="domain_events")
    op.drop_index("ix_domain_events_order_id", table_name="domain_events")
    op.drop_table("domain_events")
