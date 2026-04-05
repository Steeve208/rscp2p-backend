"""
Módulo /users — Fase 2 Bloque A.
GET /users/wallet/{address}, GET /users/stats/{address},
GET /users/ranking, PUT /users/me/profile (protegido con JWT),
GET /users/{id} (last — catch-all param route).
"""

from fastapi import APIRouter, Depends, HTTPException, Query
from sqlalchemy.orm import Session

from app.api.routes.auth import get_current_user
from app.db import get_db
from app.schemas.auth import UserResponse
from app.schemas.users import (
    ProfileUpdate,
    RankingResponse,
    UserProfilePublic,
    UserStatsResponse,
)
from app.services import users as users_service

router = APIRouter(prefix="/users", tags=["users"])


@router.get("/wallet/{address}", response_model=UserProfilePublic)
def get_user_by_wallet(
    address: str,
    db: Session = Depends(get_db),
):
    """Perfil público por dirección de wallet. 404 si no existe."""
    if not (address or str(address).strip()):
        raise HTTPException(status_code=422, detail="Address is required")
    profile = users_service.get_profile_by_wallet(db, address)
    if profile is None:
        raise HTTPException(status_code=404, detail="User not found")
    return profile


@router.get("/stats/{address}", response_model=UserStatsResponse)
def get_user_stats(
    address: str,
    db: Session = Depends(get_db),
):
    """Estadísticas por dirección de wallet. 404 si el usuario no existe."""
    if not (address or str(address).strip()):
        raise HTTPException(status_code=422, detail="Address is required")
    stats = users_service.get_stats(db, address)
    if stats is None:
        raise HTTPException(status_code=404, detail="User not found")
    return stats


@router.get("/ranking", response_model=RankingResponse)
def get_ranking(
    db: Session = Depends(get_db),
    limit: int = Query(20, ge=1, le=100),
    offset: int = Query(0, ge=0),
):
    """Ranking de usuarios por reputación, paginado."""
    return users_service.get_ranking(db, limit=limit, offset=offset)


@router.put("/me/profile", response_model=UserProfilePublic)
def update_my_profile(
    body: ProfileUpdate,
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
):
    """Actualiza el perfil del usuario autenticado. Requiere JWT."""
    profile = users_service.update_profile(db, user.id, body)
    if profile is None:
        raise HTTPException(status_code=404, detail="User not found")
    return profile


@router.get("/{user_id}", response_model=UserProfilePublic)
def get_user_by_id(
    user_id: str,
    db: Session = Depends(get_db),
):
    """Perfil público por id. 404 si no existe."""
    profile = users_service.get_profile_by_id(db, user_id)
    if profile is None:
        raise HTTPException(status_code=404, detail="User not found")
    return profile
