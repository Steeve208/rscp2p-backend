"""Launchpad submissions: project metadata, token contract, chain, admin reviewer

Revision ID: 012_launchpad_submissions
Revises: 011_production
"""
from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa

revision: str = "012_launchpad_submissions"
down_revision: Union[str, None] = "011_production"
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def _has_column(insp, table: str, col: str) -> bool:
    return any(c["name"] == col for c in insp.get_columns(table))


def upgrade() -> None:
    bind = op.get_bind()
    insp = sa.inspect(bind)
    if not insp.has_table("launchpad_submissions"):
        return

    cols = [
        ("contract_token_address", sa.String(66), {"nullable": True}),
        ("chain_id", sa.Integer(), {"nullable": True}),
        ("project_name", sa.String(256), {"nullable": True}),
        ("token_symbol", sa.String(32), {"nullable": True}),
        ("total_supply", sa.String(64), {"nullable": True}),
        ("launch_supply", sa.String(64), {"nullable": True}),
        ("logo_url", sa.String(512), {"nullable": True}),
        ("contact_email", sa.String(256), {"nullable": True}),
        ("reviewer_wallet", sa.String(66), {"nullable": True}),
    ]

    for name, typ, kw in cols:
        if not _has_column(insp, "launchpad_submissions", name):
            op.add_column("launchpad_submissions", sa.Column(name, typ, **kw))

    if bind.dialect.name == "postgresql":
        try:
            op.alter_column(
                "launchpad_submissions",
                "contract_address",
                type_=sa.String(128),
                existing_type=sa.String(66),
                existing_nullable=True,
            )
        except Exception:
            pass


def downgrade() -> None:
    bind = op.get_bind()
    insp = sa.inspect(bind)
    if not insp.has_table("launchpad_submissions"):
        return
    for name in (
        "reviewer_wallet",
        "contact_email",
        "logo_url",
        "launch_supply",
        "total_supply",
        "token_symbol",
        "project_name",
        "chain_id",
        "contract_token_address",
    ):
        if _has_column(insp, "launchpad_submissions", name):
            op.drop_column("launchpad_submissions", name)
