"""Add status column to launchpad_presales.

Revision ID: 014
Revises: 013
"""

from alembic import op
import sqlalchemy as sa

revision = "014"
down_revision = "013"
branch_labels = None
depends_on = None


def upgrade():
    op.add_column(
        "launchpad_presales",
        sa.Column("status", sa.String(32), nullable=False, server_default="active"),
    )
    op.create_index("ix_launchpad_presales_status", "launchpad_presales", ["status"])


def downgrade():
    op.drop_index("ix_launchpad_presales_status", table_name="launchpad_presales")
    op.drop_column("launchpad_presales", "status")
