"""
Capa de base de datos SQLAlchemy para el backend.
Launchpad + Marketplace (orders, escrows, disputes, users).
"""

from __future__ import annotations

import logging
from collections.abc import Iterator
from pathlib import Path

from sqlalchemy import create_engine, inspect, text
from sqlalchemy.engine.url import make_url
from sqlalchemy.orm import Session, declarative_base, sessionmaker

from app.config import settings

logger = logging.getLogger("rsc-backend")

# Raíz del repo backend (…/p2p-backend): rutas SQLite relativas no deben depender del CWD de uvicorn.
BACKEND_ROOT = Path(__file__).resolve().parent.parent


def _normalize_sqlite_url(url: str) -> str:
    """sqlite:///./launchpad.db → siempre el mismo archivo bajo p2p-backend/."""
    try:
        u = make_url(url)
    except Exception:
        return url
    if u.drivername != "sqlite":
        return url
    db = (u.database or "").strip()
    if not db or db == ":memory:":
        return url
    path = Path(db)
    if not path.is_absolute():
        path = (BACKEND_ROOT / path).resolve()
    return str(u.set(database=str(path)))


_database_url = _normalize_sqlite_url(getattr(settings, "database_url", "sqlite:///./launchpad.db"))
_connect_args = {}
if "sqlite" in _database_url:
    _connect_args["check_same_thread"] = False
_engine_kw: dict = {"connect_args": _connect_args} if _connect_args else {}
if "postgresql" in _database_url or "postgres" in _database_url:
    _engine_kw["pool_pre_ping"] = True
    _engine_kw["pool_size"] = 5
    _engine_kw["max_overflow"] = 10

engine = create_engine(_database_url, **_engine_kw)
SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)
Base = declarative_base()

_schema_presale_status_checked = False


def ensure_launchpad_presale_status_column() -> None:
    """
    create_all no altera tablas existentes; Alembic 014 añade `status`.
    Idempotente; se ejecuta en init_db y en get_db (primera petición).
    """
    global _schema_presale_status_checked
    if _schema_presale_status_checked:
        return
    try:
        if engine.dialect.name == "sqlite":
            with engine.begin() as conn:
                row = conn.execute(
                    text(
                        "SELECT 1 FROM sqlite_master WHERE type='table' AND name='launchpad_presales'"
                    )
                ).first()
                if not row:
                    return
                cols = conn.execute(text("PRAGMA table_info(launchpad_presales)")).fetchall()
                names = {c[1] for c in cols}
                if "status" in names:
                    return
                conn.execute(
                    text(
                        "ALTER TABLE launchpad_presales ADD COLUMN status VARCHAR(32) "
                        "NOT NULL DEFAULT 'active'"
                    )
                )
                conn.execute(
                    text(
                        "CREATE INDEX IF NOT EXISTS ix_launchpad_presales_status "
                        "ON launchpad_presales (status)"
                    )
                )
                logger.info("Reparación esquema: columna launchpad_presales.status añadida (SQLite).")
        else:
            insp = inspect(engine)
            if not insp.has_table("launchpad_presales"):
                return
            col_names = {c["name"] for c in insp.get_columns("launchpad_presales")}
            if "status" in col_names:
                return
            with engine.begin() as conn:
                conn.execute(
                    text(
                        "ALTER TABLE launchpad_presales ADD COLUMN status VARCHAR(32) "
                        "NOT NULL DEFAULT 'active'"
                    )
                )
                conn.execute(
                    text(
                        "CREATE INDEX IF NOT EXISTS ix_launchpad_presales_status "
                        "ON launchpad_presales (status)"
                    )
                )
                logger.info("Reparación esquema: columna launchpad_presales.status añadida.")
    except Exception:
        logger.exception("No se pudo asegurar launchpad_presales.status; revise la base de datos.")
    finally:
        _schema_presale_status_checked = True


def get_db() -> Iterator[Session]:
    ensure_launchpad_presale_status_column()
    db = SessionLocal()
    try:
        yield db
    finally:
        db.close()
