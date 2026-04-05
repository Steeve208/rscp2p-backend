"""
Servicio de escrow con persistencia en base de datos (SQLAlchemy).
Creación y actualización de estado vía OrderAggregate; lecturas sin cambios.
On-chain verification via chain_verifier (skipped if RPC_URL not set).
"""

import logging
from datetime import datetime, timezone

from fastapi import HTTPException
from sqlalchemy import select
from sqlalchemy.orm import Session

from app.models.marketplace import EscrowModel, OrderModel
from app.repositories.order_repository import OrderRepository
from app.schemas.auth import UserResponse
from app.services.chain_verifier import ChainVerificationError, verify_transaction
from app.services.domain_events import persist_domain_events
from app.services.ledger_service import (
    create_balanced_entries,
    entries_for_release,
    entries_for_refund,
)
from app.schemas.escrow import Escrow, EscrowStatus

logger = logging.getLogger("rsc-backend")


def _dt_iso(dt: datetime | None) -> str | None:
    if dt is None:
        return None
    return dt.astimezone(timezone.utc).isoformat().replace("+00:00", "Z")


def _model_to_escrow(m: EscrowModel) -> Escrow:
    return Escrow(
        id=m.id,
        orderId=m.order_id,
        escrowId=m.external_escrow_id,
        contractAddress=m.contract_address,
        cryptoAmount=m.crypto_amount,
        cryptoCurrency=m.crypto_currency,
        status=m.status,
        createTransactionHash=m.create_tx_hash,
        releaseTransactionHash=m.release_tx_hash,
        refundTransactionHash=m.refund_tx_hash,
        lockedAt=_dt_iso(m.locked_at),
        releasedAt=_dt_iso(m.released_at),
        refundedAt=_dt_iso(m.refunded_at),
        createdAt=_dt_iso(m.created_at) or "",
        updatedAt=_dt_iso(m.updated_at) or "",
    )


async def create_escrow(
    db: Session,
    *,
    user: UserResponse,
    order_id: str,
    external_escrow_id: str,
    contract_address: str,
    crypto_amount: str,
    crypto_currency: str,
    create_tx_hash: str | None = None,
) -> Escrow:
    """Crea y enlaza escrow a la orden. Solo el comprador de la orden puede crear el escrow."""
    if create_tx_hash:
        try:
            await verify_transaction(
                create_tx_hash,
                expected_contract=contract_address,
            )
        except ChainVerificationError as exc:
            raise HTTPException(status_code=400, detail=f"On-chain verification failed: {exc}")

    repo = OrderRepository(db)
    agg = repo.get_for_update(order_id)
    if agg is None:
        raise HTTPException(status_code=404, detail="Order not found")
    if agg.order.buyer_id != user.id:
        raise HTTPException(
            status_code=403,
            detail="Only the buyer of the order can attach an escrow",
        )
    agg.attach_escrow(
        external_escrow_id=external_escrow_id,
        contract_address=contract_address,
        crypto_amount=crypto_amount,
        crypto_currency=crypto_currency,
        create_tx_hash=create_tx_hash,
    )
    repo.save(agg)
    persist_domain_events(db, agg.pull_domain_events())
    db.commit()
    if agg.escrow is not None:
        db.refresh(agg.escrow)
    return _model_to_escrow(agg.escrow)


def assert_user_can_view_escrow_for_order(db: Session, user: UserResponse, order_id: str) -> None:
    """Solo comprador o vendedor de la orden pueden ver datos de escrow."""
    m = db.get(OrderModel, order_id)
    if m is None:
        raise HTTPException(status_code=404, detail="Order not found")
    if m.seller_id != user.id and m.buyer_id != user.id:
        raise HTTPException(
            status_code=403,
            detail="Only the buyer or seller of the order can view escrow",
        )


def get_escrow_by_id(db: Session, escrow_id: str) -> Escrow | None:
    m = db.get(EscrowModel, escrow_id)
    if m is None:
        return None
    return _model_to_escrow(m)


def get_escrow_by_order_id(db: Session, order_id: str) -> Escrow | None:
    m = db.scalar(select(EscrowModel).where(EscrowModel.order_id == order_id).limit(1))
    if m is None:
        return None
    return _model_to_escrow(m)


def get_escrow_by_external_id(db: Session, external_id: str) -> Escrow | None:
    m = db.scalar(select(EscrowModel).where(EscrowModel.external_escrow_id == external_id).limit(1))
    if m is None:
        return None
    return _model_to_escrow(m)


async def update_escrow(
    db: Session,
    escrow_id: str,
    *,
    user: UserResponse,
    is_admin: bool = False,
    status: EscrowStatus | None = None,
    create_tx_hash: str | None = None,
    release_tx_hash: str | None = None,
    refund_tx_hash: str | None = None,
    released_at: str | None = None,
    refunded_at: str | None = None,
) -> Escrow | None:
    tx_to_verify = release_tx_hash or refund_tx_hash or create_tx_hash
    if tx_to_verify:
        try:
            await verify_transaction(tx_to_verify)
        except ChainVerificationError as exc:
            raise HTTPException(status_code=400, detail=f"On-chain verification failed: {exc}")

    m = db.get(EscrowModel, escrow_id)
    if m is None:
        return None
    repo = OrderRepository(db)
    agg = repo.get_for_update(m.order_id)
    if agg is None:
        return None

    is_participant = user.id == agg.order.seller_id or user.id == agg.order.buyer_id
    if not is_participant and not is_admin:
        raise HTTPException(
            status_code=403,
            detail="Only participants or admins can update the escrow",
        )

    order_is_disputed = agg.order.status == "DISPUTED"
    if order_is_disputed and status in ("RELEASED", "REFUNDED") and not is_admin:
        raise HTTPException(
            status_code=403,
            detail="Only an admin can resolve a disputed order",
        )

    if status == "RELEASED":
        agg.resolve_dispute_release(release_tx_hash=release_tx_hash, released_at=released_at)
        release_entries = entries_for_release(
            agg.order.id,
            agg.order.crypto_amount,
            agg.order.crypto_currency,
            release_tx_hash,
        )
        create_balanced_entries(db, agg.order.id, release_entries)
    elif status == "REFUNDED":
        agg.resolve_dispute_refund(refund_tx_hash=refund_tx_hash, refunded_at=refunded_at)
        refund_entries = entries_for_refund(
            agg.order.id,
            agg.order.crypto_amount,
            agg.order.crypto_currency,
            refund_tx_hash,
        )
        create_balanced_entries(db, agg.order.id, refund_entries)
    elif status == "FUNDED" or create_tx_hash is not None:
        agg.record_escrow_locked(create_tx_hash=create_tx_hash)
    else:
        raise HTTPException(
            status_code=400,
            detail="Provide status RELEASED, REFUNDED or FUNDED/create_tx_hash",
        )
    repo.save(agg)
    persist_domain_events(db, agg.pull_domain_events())
    db.commit()
    db.refresh(agg.order)
    if agg.escrow is not None:
        db.refresh(agg.escrow)
    return _model_to_escrow(agg.escrow)
