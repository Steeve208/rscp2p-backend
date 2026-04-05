"""
Endpoint para recibir eventos de depósito desde un blockchain watcher.
Protegido por HMAC-SHA256 (X-Webhook-Signature), no por JWT de usuario.
"""

import hashlib
import hmac
import logging

from fastapi import APIRouter, Depends, HTTPException, Request
from sqlalchemy.orm import Session

from app.config import settings
from app.db import get_db
from app.schemas.deposit import DepositEventBody, DepositResultResponse
from app.services.deposit_processor import DepositEventPayload, process_deposit_event
from app.websocket.socketio import notify_order_updated

logger = logging.getLogger("rsc-backend")

router = APIRouter(prefix="/deposits", tags=["deposits"])


async def verify_webhook_signature(request: Request) -> None:
    """Validate X-Webhook-Signature header using HMAC-SHA256 of raw body."""
    if not settings.webhook_secret:
        if settings.is_production:
            raise HTTPException(status_code=503, detail="Webhook secret not configured")
        return

    signature = request.headers.get("X-Webhook-Signature")
    if not signature:
        raise HTTPException(status_code=401, detail="Missing webhook signature")

    body = await request.body()
    expected = hmac.new(
        settings.webhook_secret.encode(), body, hashlib.sha256
    ).hexdigest()

    if not hmac.compare_digest(signature, expected):
        logger.warning("Invalid webhook signature from %s", request.client.host if request.client else "unknown")
        raise HTTPException(status_code=403, detail="Invalid webhook signature")


@router.post("", response_model=DepositResultResponse, dependencies=[Depends(verify_webhook_signature)])
def receive_deposit(body: DepositEventBody, db: Session = Depends(get_db)):
    payload = DepositEventPayload(
        order_id=body.orderId,
        tx_hash=body.txHash,
        amount=body.amount,
        currency=body.currency,
        external_escrow_id=body.externalEscrowId,
        contract_address=body.contractAddress,
        idempotency_key=body.idempotencyKey,
    )
    result = process_deposit_event(db, payload)

    if not result.already_processed and not result.rejected:
        from app.services.orders import get_order_by_id
        order = get_order_by_id(db, result.order_id)
        if order:
            order_dict = order.model_dump(mode="json")
            seller = order_dict.get("seller") or {}
            buyer = order_dict.get("buyer") or {}
            notify_order_updated(
                order_dict, "order:updated",
                seller_id=seller.get("wallet_address", "").lower() if seller else None,
                buyer_id=buyer.get("wallet_address", "").lower() if buyer else None,
            )

    return DepositResultResponse(
        orderId=result.order_id,
        orderStatus=result.order_status,
        escrowFunded=result.escrow_funded,
        alreadyProcessed=result.already_processed,
        rejected=result.rejected,
        rejectionReason=result.rejection_reason,
    )
