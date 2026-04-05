# p2p-backend/app/schemas/users.py
# Schemas para el módulo /users — Fase 2 Bloque A.
# Alineado con UserModel: id (str), wallet_address, reputation_score; perfil opcional (nickname, etc.) si existe.

from __future__ import annotations

from typing import Optional

from pydantic import BaseModel, Field


# ----- Response -----

class UserProfilePublic(BaseModel):
    """Perfil público (GET /users/{id}, GET /users/wallet/{address})."""
    id: str
    wallet_address: str
    reputation_score: float = 0.0
    nickname: Optional[str] = None
    avatar_url: Optional[str] = None
    bio: Optional[str] = None

    class Config:
        from_attributes = True


class UserStatsResponse(BaseModel):
    """Estadísticas por address (GET /users/stats/{address})."""
    wallet_address: str
    total_trades: int = 0
    successful_trades: int = 0
    success_rate: float = 0.0
    volume_eur: float = 0.0


class RankingEntry(BaseModel):
    """Entrada del ranking (GET /users/ranking)."""
    rank: int
    user_id: str
    wallet_address: str
    nickname: Optional[str] = None
    score: float = 0.0


class RankingResponse(BaseModel):
    """Lista paginada del ranking."""
    items: list[RankingEntry]
    total: int
    limit: int
    offset: int


# ----- Request (PUT /users/me/profile) -----

class ProfileUpdate(BaseModel):
    """Campos editables del perfil (solo los enviados se actualizan)."""
    nickname: Optional[str] = Field(None, min_length=1, max_length=64)
    avatar_url: Optional[str] = Field(None, max_length=512)
    bio: Optional[str] = Field(None, max_length=500)
