"""
Capa de base de datos SQLAlchemy para el backend.
Launchpad + Marketplace (orders, escrows, disputes, users).
"""

from sqlalchemy import create_engine
from sqlalchemy.orm import Session, declarative_base, sessionmaker

from app.config import settings

_database_url = getattr(settings, "database_url", "sqlite:///./launchpad.db")
_connect_args = {}
if "sqlite" in _database_url:
    _connect_args["check_same_thread"] = False
_engine_kw: dict = {"connect_args": _connect_args} if _connect_args else {}
# PostgreSQL: pool para producción
if "postgresql" in _database_url or "postgres" in _database_url:
    _engine_kw["pool_pre_ping"] = True
    _engine_kw["pool_size"] = 5
    _engine_kw["max_overflow"] = 10

engine = create_engine(_database_url, **_engine_kw)
SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)
Base = declarative_base()


def get_db() -> Session:
    db = SessionLocal()
    try:
        yield db
    finally:
        db.close()
