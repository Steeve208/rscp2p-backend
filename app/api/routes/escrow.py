"""
Rutas de escrow para marketplace.
Autenticación obligatoria: solo comprador puede crear escrow; solo comprador o vendedor pueden actualizar.
"""

from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.orm import Session

from app.api.routes.auth import get_current_user
from app.config import settings
from app.db import get_db
from app.schemas.auth import UserResponse
from app.schemas.escrow import CreateEscrowBody, Escrow, UpdateEscrowBody
from app.services.escrow import (
    assert_user_can_view_escrow_for_order,
    create_escrow,
    get_escrow_by_external_id,
    get_escrow_by_id,
    get_escrow_by_order_id,
    update_escrow,
)

router = APIRouter(prefix="/escrow", tags=["escrow"])


@router.post("", response_model=Escrow)
async def post_escrow(
    body: CreateEscrowBody,
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
):
    return await create_escrow(
        db,
        user=user,
        order_id=body.orderId,
        external_escrow_id=body.escrowId,
        contract_address=body.contractAddress,
        crypto_amount=body.cryptoAmount,
        crypto_currency=body.cryptoCurrency,
        create_tx_hash=body.createTransactionHash,
    )


# Rutas estáticas antes de /{escrow_id} para que "order" y "blockchain" no se interpreten como UUID.
@router.get("/order/{order_id}", response_model=Escrow)
def get_escrow_by_order(
    order_id: str,
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
):
    assert_user_can_view_escrow_for_order(db, user, order_id)
    escrow = get_escrow_by_order_id(db, order_id)
    if escrow is None:
        raise HTTPException(status_code=404, detail="Escrow not found")
    return escrow


@router.get("/blockchain/{escrow_id}", response_model=Escrow)
def get_escrow_by_blockchain_id(
    escrow_id: str,
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
):
    escrow = get_escrow_by_external_id(db, escrow_id)
    if escrow is None:
        raise HTTPException(status_code=404, detail="Escrow not found")
    assert_user_can_view_escrow_for_order(db, user, escrow.orderId)
    return escrow


@router.get("/{escrow_id}", response_model=Escrow)
def get_escrow(
    escrow_id: str,
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
):
    escrow = get_escrow_by_id(db, escrow_id)
    if escrow is None:
        raise HTTPException(status_code=404, detail="Escrow not found")
    assert_user_can_view_escrow_for_order(db, user, escrow.orderId)
    return escrow


@router.put("/{escrow_id}", response_model=Escrow)
async def put_escrow(
    escrow_id: str,
    body: UpdateEscrowBody,
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
):
    user_is_admin = bool(
        settings.admin_wallet_set and user.walletAddress.lower() in settings.admin_wallet_set
    )
    escrow = await update_escrow(
        db,
        escrow_id,
        user=user,
        is_admin=user_is_admin,
        status=body.status,
        create_tx_hash=body.createTransactionHash,
        release_tx_hash=body.releaseTransactionHash,
        refund_tx_hash=body.refundTransactionHash,
        released_at=body.releasedAt,
        refunded_at=body.refundedAt,
    )
    if escrow is None:
        raise HTTPException(status_code=404, detail="Escrow not found")
    return escrow
