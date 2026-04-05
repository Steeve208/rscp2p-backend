"""
Servicio del módulo /users — Fase 2 Bloque A.
Solo consulta/actualiza UserModel y OrderModel; sin mocks, sin datos en memoria.
"""

from __future__ import annotations

from sqlalchemy import Float, func, or_, select
from sqlalchemy.orm import Session
from sqlalchemy.sql import cast

from app.models.marketplace import OrderModel, UserModel
from app.schemas.users import (
    ProfileUpdate,
    RankingEntry,
    RankingResponse,
    UserProfilePublic,
    UserStatsResponse,
)


def _user_to_public(u: UserModel) -> UserProfilePublic:
    """Mapea UserModel a UserProfilePublic. Campos opcionales con getattr para forward-compat."""
    return UserProfilePublic(
        id=u.id,
        wallet_address=u.wallet_address,
        reputation_score=float(u.reputation_score),
        nickname=getattr(u, "nickname", None),
        avatar_url=getattr(u, "avatar_url", None),
        bio=getattr(u, "bio", None),
    )


def get_by_id(db: Session, user_id: str) -> UserModel | None:
    """Obtiene usuario por id. None si no existe o no está activo."""
    u = db.scalar(select(UserModel).where(UserModel.id == user_id).limit(1))
    if u is None or not getattr(u, "is_active", True):
        return None
    return u


def get_by_wallet(db: Session, address: str) -> UserModel | None:
    """Obtiene usuario por wallet_address (normalizado lower). None si no existe."""
    addr = (address or "").strip().lower()
    if not addr:
        return None
    u = db.scalar(
        select(UserModel).where(UserModel.wallet_address == addr).limit(1)
    )
    if u is None or not getattr(u, "is_active", True):
        return None
    return u


def get_profile_by_id(db: Session, user_id: str) -> UserProfilePublic | None:
    """Perfil público por id. None si no existe."""
    u = get_by_id(db, user_id)
    return _user_to_public(u) if u else None


def get_profile_by_wallet(db: Session, address: str) -> UserProfilePublic | None:
    """Perfil público por wallet_address. None si no existe."""
    u = get_by_wallet(db, address)
    return _user_to_public(u) if u else None


def get_stats(db: Session, address: str) -> UserStatsResponse | None:
    """
    Estadísticas por wallet_address.
    Deriva de OrderModel (seller_wallet/buyer_wallet). None si el usuario no existe.
    """
    u = get_by_wallet(db, address)
    if u is None:
        return None
    addr = u.wallet_address

    # Órdenes donde el usuario es seller o buyer (por wallet)
    wallet_filter = or_(
        OrderModel.seller_wallet == addr,
        OrderModel.buyer_wallet == addr,
    )
    total_trades = db.scalar(
        select(func.count()).select_from(OrderModel).where(wallet_filter)
    ) or 0

    successful_trades = db.scalar(
        select(func.count())
        .select_from(OrderModel)
        .where(wallet_filter, OrderModel.status == "RELEASED")
    ) or 0

    success_rate = (successful_trades / total_trades) if total_trades else 0.0

    # Volumen: suma fiat_amount de órdenes completadas (como float)
    vol_q = select(func.sum(cast(OrderModel.fiat_amount, Float))).where(
        or_(
            OrderModel.seller_wallet == addr,
            OrderModel.buyer_wallet == addr,
        ),
        OrderModel.status == "RELEASED",
    )
    try:
        volume_eur = float(db.scalar(vol_q) or 0)
    except (TypeError, ValueError):
        volume_eur = 0.0

    return UserStatsResponse(
        wallet_address=addr,
        total_trades=total_trades,
        successful_trades=successful_trades,
        success_rate=round(success_rate, 2),
        volume_eur=round(volume_eur, 2),
    )


def get_ranking(
    db: Session,
    limit: int = 20,
    offset: int = 0,
) -> RankingResponse:
    """Ranking por reputation_score desc, paginado. Solo usuarios activos."""
    count_q = select(func.count()).select_from(UserModel).where(
        UserModel.is_active == True
    )
    total = db.scalar(count_q) or 0

    q = (
        select(UserModel)
        .where(UserModel.is_active == True)
        .order_by(UserModel.reputation_score.desc(), UserModel.id.asc())
        .offset(offset)
        .limit(max(1, min(limit, 100)))
    )
    rows = db.scalars(q).all()

    items = [
        RankingEntry(
            rank=offset + i + 1,
            user_id=u.id,
            wallet_address=u.wallet_address,
            nickname=getattr(u, "nickname", None),
            score=float(u.reputation_score),
        )
        for i, u in enumerate(rows)
    ]
    return RankingResponse(
        items=items,
        total=total,
        limit=limit,
        offset=offset,
    )


def update_profile(
    db: Session,
    user_id: str,
    body: ProfileUpdate,
) -> UserProfilePublic | None:
    """
    Actualiza perfil del usuario por id.
    Solo actualiza atributos que existan en el modelo (forward-compat).
    None si el usuario no existe.
    """
    u = get_by_id(db, user_id)
    if u is None:
        return None

    if body.nickname is not None and hasattr(u, "nickname"):
        u.nickname = body.nickname[:64] if body.nickname else None
    if body.avatar_url is not None and hasattr(u, "avatar_url"):
        u.avatar_url = body.avatar_url[:512] if body.avatar_url else None
    if body.bio is not None and hasattr(u, "bio"):
        u.bio = body.bio[:500] if body.bio else None

    db.commit()
    db.refresh(u)
    return _user_to_public(u)
