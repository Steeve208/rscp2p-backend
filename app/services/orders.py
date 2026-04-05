"""
Servicio de órdenes con persistencia en base de datos (SQLAlchemy).
Transiciones de estado vía OrderAggregate (app.domain.order_aggregate) y OrderRepository.
"""

import logging
import math
from datetime import datetime, timezone

from fastapi import HTTPException
from sqlalchemy import func, or_, select
from sqlalchemy.orm import Session

from app.domain.order_aggregate import OrderAggregate
from app.models.alerts import AlertModel
from app.models.marketplace import DisputeModel, OrderModel, UserModel
from app.repositories.order_repository import OrderRepository
from app.services.domain_events import persist_domain_events
from app.services.ledger_service import (
    create_balanced_entries,
    entries_for_deposit,
    entries_for_release,
    entries_for_refund,
)
from app.schemas.auth import UserResponse
from app.schemas.common import PaginatedResponse
from app.schemas.order import Order, OrderBuyer, OrderSeller, OrderStatus

logger = logging.getLogger("rsc-backend")

REPUTATION_INCREMENT_PER_TRADE = 0.5

DISPUTE_OPEN_STATUSES = frozenset({"OPEN", "IN_REVIEW", "ESCALATED"})


def _create_order_alert(
    db: Session,
    user_id: str,
    alert_type: str,
    title: str,
    message: str,
    severity: str = "medium",
    order_id: str | None = None,
) -> None:
    """Create a persistent alert for an order lifecycle event."""
    import json
    from uuid import uuid4
    data = json.dumps({"orderId": order_id}) if order_id else None
    db.add(AlertModel(
        id=str(uuid4()),
        user_id=user_id,
        type=alert_type,
        title=title,
        message=message,
        severity=severity,
        data=data,
        read=False,
    ))


def _update_reputation(db: Session, user_id: str, increment: float) -> None:
    """Increment a user's reputation score after a successful trade."""
    user = db.scalar(select(UserModel).where(UserModel.id == user_id).limit(1))
    if user:
        user.reputation_score = (user.reputation_score or 0.0) + increment


def _dt_iso(dt: datetime | None) -> str | None:
    if dt is None:
        return None
    return dt.astimezone(timezone.utc).isoformat().replace("+00:00", "Z")


def _model_to_order(m: OrderModel) -> Order:
    rep_seller = 0.0
    if m.seller_reputation:
        try:
            rep_seller = float(m.seller_reputation)
        except (TypeError, ValueError):
            pass
    rep_buyer = 0.0
    if m.buyer_reputation:
        try:
            rep_buyer = float(m.buyer_reputation)
        except (TypeError, ValueError):
            pass
    seller = OrderSeller(
        id=m.seller_id,
        wallet_address=m.seller_wallet or "",
        reputation_score=rep_seller,
    )
    buyer = None
    if m.buyer_id:
        buyer = OrderBuyer(
            id=m.buyer_id,
            wallet_address=m.buyer_wallet or "",
            reputation_score=rep_buyer,
        )
    return Order(
        id=m.id,
        sellerId=m.seller_id,
        buyerId=m.buyer_id,
        seller=seller,
        buyer=buyer,
        cryptoCurrency=m.crypto_currency,
        cryptoAmount=m.crypto_amount,
        fiatCurrency=m.fiat_currency,
        fiatAmount=m.fiat_amount,
        pricePerUnit=m.price_per_unit or "",
        status=m.status,
        escrowId=m.escrow_id,
        paymentMethod=m.payment_method,
        terms=m.terms,
        expiresAt=_dt_iso(m.expires_at),
        acceptedAt=_dt_iso(m.accepted_at),
        completedAt=_dt_iso(m.completed_at),
        cancelledAt=_dt_iso(m.cancelled_at),
        cancelledBy=m.cancelled_by,
        disputedAt=_dt_iso(m.disputed_at),
        createdAt=_dt_iso(m.created_at) or "",
        updatedAt=_dt_iso(m.updated_at) or "",
    )


def list_orders(
    db: Session,
    page: int = 1,
    limit: int = 100,
    status: OrderStatus | None = None,
    seller_id: str | None = None,
    buyer_id: str | None = None,
    crypto_currency: str | None = None,
    fiat_currency: str | None = None,
) -> PaginatedResponse[Order]:
    q = select(OrderModel).order_by(OrderModel.created_at.desc())
    if status is not None:
        q = q.where(OrderModel.status == status)
    if seller_id is not None:
        q = q.where(OrderModel.seller_id == seller_id)
    if buyer_id is not None:
        q = q.where(OrderModel.buyer_id == buyer_id)
    if crypto_currency is not None:
        q = q.where(OrderModel.crypto_currency == crypto_currency)
    if fiat_currency is not None:
        q = q.where(OrderModel.fiat_currency == fiat_currency)

    count_q = select(func.count()).select_from(OrderModel)
    if status is not None:
        count_q = count_q.where(OrderModel.status == status)
    if seller_id is not None:
        count_q = count_q.where(OrderModel.seller_id == seller_id)
    if buyer_id is not None:
        count_q = count_q.where(OrderModel.buyer_id == buyer_id)
    if crypto_currency is not None:
        count_q = count_q.where(OrderModel.crypto_currency == crypto_currency)
    if fiat_currency is not None:
        count_q = count_q.where(OrderModel.fiat_currency == fiat_currency)
    total = db.scalar(count_q) or 0
    all_rows = list(db.scalars(q.offset((page - 1) * limit).limit(limit)).all())
    total_pages = math.ceil(total / limit) if limit > 0 else 0
    data = [_model_to_order(m) for m in all_rows]
    return PaginatedResponse(
        data=data,
        total=total,
        page=page,
        limit=limit,
        totalPages=total_pages,
    )


def list_my_orders(
    db: Session,
    user_id: str,
    role: str = "both",
    status: OrderStatus | None = None,
    page: int = 1,
    limit: int = 100,
) -> PaginatedResponse[Order]:
    q = select(OrderModel)
    if role == "seller":
        q = q.where(OrderModel.seller_id == user_id)
    elif role == "buyer":
        q = q.where(OrderModel.buyer_id == user_id)
    else:
        q = q.where(or_(OrderModel.seller_id == user_id, OrderModel.buyer_id == user_id))
    if status is not None:
        q = q.where(OrderModel.status == status)
    q = q.order_by(OrderModel.created_at.desc())
    total_q = select(func.count()).select_from(OrderModel)
    if role == "seller":
        total_q = total_q.where(OrderModel.seller_id == user_id)
    elif role == "buyer":
        total_q = total_q.where(OrderModel.buyer_id == user_id)
    else:
        total_q = total_q.where(or_(OrderModel.seller_id == user_id, OrderModel.buyer_id == user_id))
    if status is not None:
        total_q = total_q.where(OrderModel.status == status)
    total = db.scalar(total_q) or 0
    total_pages = math.ceil(total / limit) if limit > 0 else 0
    all_rows = list(db.scalars(q.offset((page - 1) * limit).limit(limit)).all())
    data = [_model_to_order(m) for m in all_rows]
    return PaginatedResponse(
        data=data,
        total=total,
        page=page,
        limit=limit,
        totalPages=total_pages,
    )


def get_order_by_id(db: Session, order_id: str) -> Order | None:
    m = db.get(OrderModel, order_id)
    if m is None:
        return None
    return _model_to_order(m)


def _count_open_disputes(db: Session, order_id: str) -> int:
    n = db.scalar(
        select(func.count()).select_from(DisputeModel).where(
            DisputeModel.order_id == order_id,
            DisputeModel.status.in_(DISPUTE_OPEN_STATUSES),
        )
    )
    return int(n or 0)


def create_order(
    db: Session,
    user: UserResponse,
    *,
    crypto_currency: str,
    crypto_amount: str,
    fiat_currency: str,
    fiat_amount: str,
    price_per_unit: str,
    payment_method: str,
    terms: str | None = None,
    expires_at: str | None = None,
) -> Order:
    now = datetime.now(timezone.utc)
    m = OrderModel(
        seller_id=user.id,
        seller_wallet=user.walletAddress,
        seller_reputation=str(user.reputationScore),
        buyer_id=None,
        buyer_wallet=None,
        buyer_reputation=None,
        crypto_currency=crypto_currency,
        crypto_amount=crypto_amount,
        fiat_currency=fiat_currency,
        fiat_amount=fiat_amount,
        price_per_unit=price_per_unit,
        status="CREATED",
        payment_method=payment_method,
        terms=terms,
        expires_at=datetime.fromisoformat(expires_at.replace("Z", "+00:00")) if expires_at else None,
    )
    db.add(m)
    db.commit()
    db.refresh(m)
    return _model_to_order(m)


def accept_order(
    db: Session, order_id: str, buyer: UserResponse, *, commit: bool = True
) -> Order | None:
    repo = OrderRepository(db)
    agg = repo.get_for_update(order_id)
    if agg is None:
        return None
    agg.accept(
        buyer.id,
        buyer.walletAddress,
        str(buyer.reputationScore),
        buyer.id,
    )
    repo.save(agg)
    persist_domain_events(db, agg.pull_domain_events())

    pair = f"{agg.order.crypto_amount} {agg.order.crypto_currency}"
    _create_order_alert(
        db, agg.order.seller_id, "volume-spike",
        "Orden aceptada", f"Un comprador ha aceptado tu oferta de {pair}.",
        severity="medium", order_id=agg.order.id,
    )

    if commit:
        db.commit()
        db.refresh(agg.order)
    else:
        db.flush()
        db.refresh(agg.order)
    return _model_to_order(agg.order)


def mark_order_locked(
    db: Session, order_id: str, user: UserResponse, *, commit: bool = True
) -> Order | None:
    repo = OrderRepository(db)
    agg = repo.get_for_update(order_id)
    if agg is None:
        return None
    if user.id != agg.order.buyer_id:
        raise HTTPException(status_code=403, detail="Only buyer can mark order as locked")
    agg.fund_escrow(user.id)
    # Ledger: deposit double-entry when manually marking funded
    deposit_entries = entries_for_deposit(
        agg.order.id,
        agg.order.crypto_amount,
        agg.order.crypto_currency,
        agg.escrow.create_tx_hash if agg.escrow else None,
    )
    create_balanced_entries(db, agg.order.id, deposit_entries)
    repo.save(agg)
    persist_domain_events(db, agg.pull_domain_events())
    if commit:
        db.commit()
        db.refresh(agg.order)
    else:
        db.flush()
        db.refresh(agg.order)
    return _model_to_order(agg.order)


def complete_order(
    db: Session, order_id: str, user: UserResponse, *, commit: bool = True
) -> Order | None:
    repo = OrderRepository(db)
    agg = repo.get_for_update(order_id)
    if agg is None:
        return None
    role = "SELLER" if user.id == agg.order.seller_id else ("BUYER" if user.id == agg.order.buyer_id else None)
    if role is None:
        raise HTTPException(status_code=403, detail="Only seller or buyer can complete the order")
    agg.complete(role, user.id)
    release_entries = entries_for_release(
        agg.order.id,
        agg.order.crypto_amount,
        agg.order.crypto_currency,
        agg.escrow.release_tx_hash if agg.escrow else None,
    )
    create_balanced_entries(db, agg.order.id, release_entries)
    repo.save(agg)
    persist_domain_events(db, agg.pull_domain_events())

    _update_reputation(db, agg.order.seller_id, REPUTATION_INCREMENT_PER_TRADE)
    if agg.order.buyer_id:
        _update_reputation(db, agg.order.buyer_id, REPUTATION_INCREMENT_PER_TRADE)

    pair = f"{agg.order.crypto_amount} {agg.order.crypto_currency}"
    _create_order_alert(
        db, agg.order.seller_id, "price-threshold",
        "Orden completada", f"Tu venta de {pair} ha sido completada exitosamente.",
        severity="low", order_id=agg.order.id,
    )
    if agg.order.buyer_id:
        _create_order_alert(
            db, agg.order.buyer_id, "price-threshold",
            "Orden completada", f"Tu compra de {pair} ha sido completada exitosamente.",
            severity="low", order_id=agg.order.id,
        )

    if commit:
        db.commit()
        db.refresh(agg.order)
    else:
        db.flush()
        db.refresh(agg.order)
    return _model_to_order(agg.order)


def cancel_order(
    db: Session,
    order_id: str,
    user: UserResponse,
    cancelled_by: str,
    *,
    commit: bool = True,
) -> Order | None:
    repo = OrderRepository(db)
    agg = repo.get_for_update(order_id)
    if agg is None:
        return None
    if user.id != agg.order.seller_id and user.id != agg.order.buyer_id:
        raise HTTPException(status_code=403, detail="Only seller or buyer can cancel the order")
    if cancelled_by == "SELLER" and user.id != agg.order.seller_id:
        raise HTTPException(status_code=403, detail="Only seller can cancel as seller")
    if cancelled_by == "BUYER" and user.id != agg.order.buyer_id:
        raise HTTPException(status_code=403, detail="Only buyer can cancel as buyer")
    role = "SELLER" if user.id == agg.order.seller_id else "BUYER"
    agg.refund(cancelled_by, role, user.id)
    # Ledger: refund double-entry (escrow -> buyer_balance)
    refund_entries = entries_for_refund(
        agg.order.id,
        agg.order.crypto_amount,
        agg.order.crypto_currency,
        agg.escrow.refund_tx_hash if agg.escrow else None,
    )
    create_balanced_entries(db, agg.order.id, refund_entries)
    repo.save(agg)
    persist_domain_events(db, agg.pull_domain_events())
    if commit:
        db.commit()
        db.refresh(agg.order)
    else:
        db.flush()
        db.refresh(agg.order)
    return _model_to_order(agg.order)


def dispute_order(
    db: Session, order_id: str, user: UserResponse, *, commit: bool = True
) -> Order | None:
    if _count_open_disputes(db, order_id) > 0:
        raise HTTPException(status_code=409, detail="Order already has an open dispute")
    repo = OrderRepository(db)
    agg = repo.get_for_update(order_id)
    if agg is None:
        return None
    role = "SELLER" if user.id == agg.order.seller_id else ("BUYER" if user.id == agg.order.buyer_id else None)
    if role is None:
        raise HTTPException(status_code=403, detail="Only seller or buyer can open a dispute")
    agg.open_dispute(role, user.id)
    repo.save(agg)
    persist_domain_events(db, agg.pull_domain_events())

    counterpart = agg.order.buyer_id if user.id == agg.order.seller_id else agg.order.seller_id
    if counterpart:
        _create_order_alert(
            db, counterpart, "manipulation-detected",
            "Disputa abierta", "Se ha abierto una disputa en una de tus órdenes.",
            severity="high", order_id=agg.order.id,
        )

    if commit:
        db.commit()
        db.refresh(agg.order)
    else:
        db.flush()
        db.refresh(agg.order)
    return _model_to_order(agg.order)


def get_order_status(db: Session, order_id: str) -> dict | None:
    m = db.get(OrderModel, order_id)
    if m is None:
        return None
    return {
        "id": m.id,
        "status": m.status,
        "escrowId": m.escrow_id,
        "updatedAt": _dt_iso(m.updated_at),
    }


def set_order_escrow(db: Session, order_id: str, escrow_id: str) -> Order | None:
    from app.models.marketplace import EscrowModel
    escrow = db.get(EscrowModel, escrow_id)
    if escrow is None:
        return None
    repo = OrderRepository(db)
    agg = repo.get_for_update(order_id)
    if agg is None:
        return None
    agg.link_escrow(escrow)
    repo.save(agg)
    persist_domain_events(db, agg.pull_domain_events())
    db.commit()
    db.refresh(agg.order)
    return _model_to_order(agg.order)


def get_all_orders_for_events(db: Session) -> list[Order]:
    rows = db.scalars(select(OrderModel).order_by(OrderModel.created_at.desc())).all()
    return [_model_to_order(m) for m in rows]
