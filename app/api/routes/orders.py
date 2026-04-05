"""
Órdenes Marketplace.
Incluye CRUD base + transiciones de estado necesarias para el frontend.
Mutaciones (accept, mark-locked, complete, cancel, dispute) son idempotentes vía header Idempotency-Key.
"""

from fastapi import APIRouter, Depends, HTTPException, Query
from fastapi.responses import JSONResponse
from sqlalchemy.orm import Session

from app.api.deps import require_idempotency_key
from app.api.routes.auth import get_current_user
from app.db import get_db
from app.schemas.auth import UserResponse
from app.websocket.socketio import notify_order_updated
from app.schemas.order import AcceptOrderBody, CreateOrderBody, Order, OrderStatus
from app.services.idempotency_service import (
    ENDPOINT_ACCEPT,
    ENDPOINT_CANCEL,
    ENDPOINT_COMPLETE,
    ENDPOINT_DISPUTE,
    ENDPOINT_MARK_LOCKED,
    run_idempotent,
)
from app.services.orders import (
    accept_order,
    cancel_order,
    complete_order,
    create_order,
    dispute_order,
    get_order_by_id,
    list_my_orders,
    list_orders,
    mark_order_locked,
)

router = APIRouter(prefix="/orders", tags=["orders"])


def _order_to_dict(order: Order) -> dict:
    return order.model_dump(mode="json")


def _notify(content: dict, event: str = "order:updated") -> None:
    seller = content.get("seller") or {}
    buyer = content.get("buyer") or {}
    notify_order_updated(
        content, event,
        seller_id=seller.get("wallet_address", "").lower() if seller else None,
        buyer_id=buyer.get("wallet_address", "").lower() if buyer else None,
    )


def _assert_order_participant(order: Order, user: UserResponse) -> None:
    if order.sellerId != user.id and order.buyerId != user.id:
        raise HTTPException(status_code=403, detail="Not a participant of this order")


@router.get("", response_model=None)
def get_orders(
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
    page: int = Query(1, ge=1),
    limit: int = Query(50, ge=1, le=100),
    status: OrderStatus | None = None,
    sellerId: str | None = None,
    buyerId: str | None = None,
    cryptoCurrency: str | None = None,
    fiatCurrency: str | None = None,
):
    if not status:
        status = "CREATED"
    result = list_orders(
        db,
        page=page,
        limit=limit,
        status=status,
        seller_id=sellerId,
        buyer_id=buyerId,
        crypto_currency=cryptoCurrency,
        fiat_currency=fiatCurrency,
    )
    return {
        "data": result.data,
        "total": result.total,
        "page": result.page,
        "limit": result.limit,
        "totalPages": result.totalPages,
    }


@router.post("", response_model=Order)
def post_order(body: CreateOrderBody, db: Session = Depends(get_db), user: UserResponse = Depends(get_current_user)):
    order = create_order(
        db,
        user,
        crypto_currency=body.cryptoCurrency,
        crypto_amount=body.cryptoAmount,
        fiat_currency=body.fiatCurrency,
        fiat_amount=body.fiatAmount,
        price_per_unit=body.pricePerUnit,
        payment_method=body.paymentMethod,
        terms=body.terms,
        expires_at=body.expiresAt,
    )
    order_dict = _order_to_dict(order)
    _notify(order_dict, "order:created")
    return order


@router.get("/me", response_model=None)
def get_orders_me(
    db: Session = Depends(get_db),
    role: str = Query("both", pattern="^(seller|buyer|both)$"),
    status: OrderStatus | None = None,
    page: int = Query(1, ge=1),
    limit: int = Query(50, ge=1, le=100),
    user: UserResponse = Depends(get_current_user),
):
    result = list_my_orders(db, user_id=user.id, role=role, status=status, page=page, limit=limit)
    return {
        "data": result.data,
        "total": result.total,
        "page": result.page,
        "limit": result.limit,
        "totalPages": result.totalPages,
    }


@router.get("/{order_id}/status")
def get_order_status_by_id(
    order_id: str,
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
):
    order = get_order_by_id(db, order_id)
    if order is None:
        raise HTTPException(status_code=404, detail="Order not found")
    _assert_order_participant(order, user)
    return {
        "id": order.id,
        "status": order.status,
        "escrowId": order.escrowId,
        "updatedAt": order.updatedAt,
    }


@router.put("/{order_id}/accept")
def put_accept_order(
    order_id: str,
    body: AcceptOrderBody,
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
    idempotency_key: str = Depends(require_idempotency_key),
):
    def run() -> Order:
        order = accept_order(db, order_id, user, commit=False)
        if order is None:
            raise HTTPException(status_code=404, detail="Order not found")
        return order

    status, content = run_idempotent(
        db, idempotency_key, ENDPOINT_ACCEPT, order_id, run, _order_to_dict
    )
    if status == 200:
        _notify(content)
    return JSONResponse(content=content, status_code=status)


@router.put("/{order_id}/mark-locked")
def put_mark_locked(
    order_id: str,
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
    idempotency_key: str = Depends(require_idempotency_key),
):
    def run() -> Order:
        order = mark_order_locked(db, order_id, user, commit=False)
        if order is None:
            raise HTTPException(status_code=404, detail="Order not found")
        return order

    status, content = run_idempotent(
        db, idempotency_key, ENDPOINT_MARK_LOCKED, order_id, run, _order_to_dict
    )
    if status == 200:
        _notify(content)
    return JSONResponse(content=content, status_code=status)


@router.put("/{order_id}/complete")
def put_complete_order(
    order_id: str,
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
    idempotency_key: str = Depends(require_idempotency_key),
):
    def run() -> Order:
        order = complete_order(db, order_id, user, commit=False)
        if order is None:
            raise HTTPException(status_code=404, detail="Order not found")
        return order

    status, content = run_idempotent(
        db, idempotency_key, ENDPOINT_COMPLETE, order_id, run, _order_to_dict
    )
    if status == 200:
        _notify(content)
    return JSONResponse(content=content, status_code=status)


@router.put("/{order_id}/cancel")
def put_cancel_order(
    order_id: str,
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
    idempotency_key: str = Depends(require_idempotency_key),
):
    def run() -> Order:
        order = get_order_by_id(db, order_id)
        if order is None:
            raise HTTPException(status_code=404, detail="Order not found")
        cancelled_by = "SELLER" if order.sellerId == user.id else "BUYER"
        result = cancel_order(db, order_id, user, cancelled_by, commit=False)
        if result is None:
            raise HTTPException(status_code=404, detail="Order not found")
        return result

    status, content = run_idempotent(
        db, idempotency_key, ENDPOINT_CANCEL, order_id, run, _order_to_dict
    )
    if status == 200:
        _notify(content)
    return JSONResponse(content=content, status_code=status)


@router.put("/{order_id}/dispute")
def put_dispute_order(
    order_id: str,
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
    idempotency_key: str = Depends(require_idempotency_key),
):
    def run() -> Order:
        order = dispute_order(db, order_id, user, commit=False)
        if order is None:
            raise HTTPException(status_code=404, detail="Order not found")
        return order

    status, content = run_idempotent(
        db, idempotency_key, ENDPOINT_DISPUTE, order_id, run, _order_to_dict
    )
    if status == 200:
        _notify(content)
    return JSONResponse(content=content, status_code=status)


@router.get("/{order_id}", response_model=Order | None)
def get_order(
    order_id: str,
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
):
    order = get_order_by_id(db, order_id)
    if order is None:
        raise HTTPException(status_code=404, detail="Order not found")
    _assert_order_participant(order, user)
    return order
