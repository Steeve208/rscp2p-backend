"""
Alembic env: usa DATABASE_URL de app.config.
Importa todos los modelos para que metadata esté disponible.
"""

import os
import sys

# Añadir raíz del backend al path
sys.path.insert(0, os.path.realpath(os.path.join(os.path.dirname(__file__), "..")))

from logging.config import fileConfig

from alembic import context
from sqlalchemy import engine_from_config
from sqlalchemy import pool
from alembic.config import Config

from app.config import settings
from app.db import Base

# Importar todos los modelos para Base.metadata
from app.models import marketplace  # noqa: F401
from app.launchpad import models as launchpad_models  # noqa: F401

config = context.config
if config.config_file_name is not None:
    fileConfig(config.config_file_name)

target_metadata = Base.metadata

# Sobrescribir sqlalchemy.url con la de settings
config.set_main_option("sqlalchemy.url", settings.database_url.replace("%", "%%"))


def run_migrations_offline() -> None:
    """Run migrations in 'offline' mode."""
    url = config.get_main_option("sqlalchemy.url")
    context.configure(
        url=url,
        target_metadata=target_metadata,
        literal_binds=True,
        dialect_opts={"paramstyle": "named"},
    )

    with context.begin_transaction():
        context.run_migrations()


def run_migrations_online() -> None:
    """Run migrations in 'online' mode."""
    connectable = engine_from_config(
        {"sqlalchemy.url": settings.database_url},
        prefix="sqlalchemy.",
        poolclass=pool.NullPool,
    )

    with connectable.connect() as connection:
        context.configure(connection=connection, target_metadata=target_metadata)

        with context.begin_transaction():
            context.run_migrations()


if context.is_offline_mode():
    run_migrations_offline()
else:
    run_migrations_online()
