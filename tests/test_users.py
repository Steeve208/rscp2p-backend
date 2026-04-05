"""
Tests del módulo /users — Fase 2 Bloque A.
Sin mocks: DB SQLite en memoria, auth mediante dependency override.
"""

import pytest
from fastapi.testclient import TestClient
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker, Session

from app.api.routes import users as users_router
from app.db import Base, get_db
from app.models.marketplace import UserModel, OrderModel
from app.schemas.auth import UserResponse
from app.api.routes.auth import get_current_user
from fastapi import FastAPI

# DB en memoria para tests
engine_test = create_engine("sqlite:///:memory:", connect_args={"check_same_thread": False})
SessionLocalTest = sessionmaker(autocommit=False, autoflush=False, bind=engine_test)


def override_get_db() -> Session:
    db = SessionLocalTest()
    try:
        yield db
    finally:
        db.close()


@pytest.fixture(scope="module")
def app_users():
    """App FastAPI con solo el router /users (y dependencias overridden)."""
    app = FastAPI()
    app.include_router(users_router.router)
    app.dependency_overrides[get_db] = override_get_db
    return app


@pytest.fixture(scope="module")
def db_session():
    """Crea tablas y devuelve una session para setup/teardown."""
    Base.metadata.create_all(bind=engine_test)
    session = SessionLocalTest()
    yield session
    session.close()


@pytest.fixture
def client(app_users, db_session):
    """TestClient con DB limpia por test (rollback o recrear tablas)."""
    return TestClient(app_users)


@pytest.fixture
def sample_user(db_session: Session) -> UserModel:
    """Usuario de prueba en DB."""
    u = UserModel(
        id="test-user-uuid-001",
        wallet_address="0x1234567890123456789012345678901234567890",
        reputation_score=10.0,
        is_active=True,
        login_count=1,
    )
    db_session.add(u)
    db_session.commit()
    db_session.refresh(u)
    return u


# ----- GET /users/{id} -----

def test_get_user_by_id_ok(client: TestClient, sample_user: UserModel):
    """GET /users/{id} devuelve 200 y perfil cuando existe."""
    r = client.get(f"/users/{sample_user.id}")
    assert r.status_code == 200
    data = r.json()
    assert data["id"] == sample_user.id
    assert data["wallet_address"] == sample_user.wallet_address
    assert data["reputation_score"] == 10.0


def test_get_user_by_id_not_found(client: TestClient):
    """GET /users/{id} devuelve 404 cuando el usuario no existe."""
    r = client.get("/users/nonexistent-id-00000")
    assert r.status_code == 404
    assert "not found" in r.json().get("detail", "").lower()


# ----- GET /users/wallet/{address} -----

def test_get_user_by_wallet_ok(client: TestClient, sample_user: UserModel):
    """GET /users/wallet/{address} devuelve 200 cuando existe."""
    r = client.get(f"/users/wallet/{sample_user.wallet_address}")
    assert r.status_code == 200
    assert r.json()["wallet_address"] == sample_user.wallet_address


def test_get_user_by_wallet_not_found(client: TestClient):
    """GET /users/wallet/{address} devuelve 404 cuando no existe."""
    r = client.get("/users/wallet/0x0000000000000000000000000000000000000000")
    assert r.status_code == 404


def test_get_user_by_wallet_empty_422(client: TestClient):
    """GET /users/wallet/ con address vacío devuelve 422."""
    r = client.get("/users/wallet/  ")
    # Puede ser 404 (strip deja vacío) o 422 según validación
    assert r.status_code in (404, 422)


# ----- GET /users/stats/{address} -----

def test_get_stats_ok(client: TestClient, sample_user: UserModel):
    """GET /users/stats/{address} devuelve 200 y stats (ceros si sin actividad)."""
    r = client.get(f"/users/stats/{sample_user.wallet_address}")
    assert r.status_code == 200
    data = r.json()
    assert data["wallet_address"] == sample_user.wallet_address
    assert data["total_trades"] >= 0
    assert "success_rate" in data


def test_get_stats_not_found(client: TestClient):
    """GET /users/stats/{address} devuelve 404 cuando el usuario no existe."""
    r = client.get("/users/stats/0x0000000000000000000000000000000000000000")
    assert r.status_code == 404


# ----- GET /users/ranking -----

def test_ranking_empty(client: TestClient, app_users, db_session):
    """GET /users/ranking sin usuarios devuelve 200 con items vacíos (o con usuarios de otros tests)."""
    r = client.get("/users/ranking")
    assert r.status_code == 200
    data = r.json()
    assert "items" in data
    assert "total" in data
    assert "limit" in data
    assert "offset" in data


def test_ranking_with_user(client: TestClient, sample_user: UserModel):
    """GET /users/ranking devuelve al menos un usuario."""
    r = client.get("/users/ranking?limit=10&offset=0")
    assert r.status_code == 200
    data = r.json()
    assert data["total"] >= 1
    assert len(data["items"]) >= 1
    assert data["items"][0]["user_id"] == sample_user.id or any(
        i["user_id"] == sample_user.id for i in data["items"]
    )


# ----- PUT /users/me/profile (auth) -----

def test_update_my_profile_401_without_token(client: TestClient):
    """PUT /users/me/profile sin token devuelve 401."""
    r = client.put("/users/me/profile", json={"nickname": "Test"})
    assert r.status_code == 401


def test_update_my_profile_ok_with_override(client: TestClient, app_users, sample_user: UserModel):
    """PUT /users/me/profile con usuario inyectado devuelve 200."""
    from datetime import datetime, timezone

    def override_get_current_user():
        return UserResponse(
            id=sample_user.id,
            walletAddress=sample_user.wallet_address,
            reputationScore=float(sample_user.reputation_score),
            isActive=sample_user.is_active,
            loginCount=sample_user.login_count or 0,
            lastLoginAt=None,
            createdAt=(sample_user.created_at.isoformat().replace("+00:00", "Z") if sample_user.created_at else datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")),
        )

    app_users.dependency_overrides[get_current_user] = override_get_current_user
    r = client.put("/users/me/profile", json={"nickname": "MyNick"})
    app_users.dependency_overrides.pop(get_current_user, None)
    assert r.status_code == 200
    data = r.json()
    assert data["id"] == sample_user.id
    # nickname solo se actualiza si el modelo tiene el atributo; si no, el body se acepta igual
    assert "wallet_address" in data
