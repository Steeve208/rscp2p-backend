"""Add status column to launchpad_presales.

Revision ID: 014
Revises: 013
"""

from alembic import op
import sqlalchemy as sa
from sqlalchemy import inspect

revision = "014"
down_revision = "013"
branch_labels = None
depends_on = None


def upgrade():
    bind = op.get_bind()
    insp = inspect(bind)
    if not insp.has_table("launchpad_presales"):
        return
    cols = {c["name"] for c in insp.get_columns("launchpad_presales")}
    if "status" in cols:
        return
    op.add_column(
        "launchpad_presales",
        sa.Column("status", sa.String(32), nullable=False, server_default="active"),
    )
    op.create_index("ix_launchpad_presales_status", "launchpad_presales", ["status"])


def downgrade():
    bind = op.get_bind()
    insp = inspect(bind)
    if not insp.has_table("launchpad_presales"):
        return
    cols = {c["name"] for c in insp.get_columns("launchpad_presales")}
    if "status" not in cols:
        return
    ix_names = {ix["name"] for ix in insp.get_indexes("launchpad_presales")}
    if "ix_launchpad_presales_status" in ix_names:
        op.drop_index("ix_launchpad_presales_status", table_name="launchpad_presales")
    op.drop_column("launchpad_presales", "status")
