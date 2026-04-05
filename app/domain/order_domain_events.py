"""
Eventos de dominio del aggregate Order.
Cada transición relevante genera un evento que el servicio puede persistir (event log / outbox).
No dependen de DB ni de FastAPI; son estructuras de datos inmutables.
"""

from dataclasses import dataclass
from datetime import datetime
from typing import Any


@dataclass(frozen=True)
class OrderDomainEvent:
    """Base: order_id y timestamp son comunes a todos."""

    order_id: str
    occurred_at: datetime
    payload: dict[str, Any]


def order_accepted(order_id: str, buyer_id: str, occurred_at: datetime) -> OrderDomainEvent:
    return OrderDomainEvent(
        order_id=order_id,
        occurred_at=occurred_at,
        payload={"type": "OrderAccepted", "buyer_id": buyer_id},
    )


def escrow_attached(order_id: str, escrow_id: str, occurred_at: datetime) -> OrderDomainEvent:
    return OrderDomainEvent(
        order_id=order_id,
        occurred_at=occurred_at,
        payload={"type": "EscrowAttached", "escrow_id": escrow_id},
    )


def escrow_funded(
    order_id: str,
    escrow_id: str | None,
    create_tx_hash: str | None,
    occurred_at: datetime,
) -> OrderDomainEvent:
    return OrderDomainEvent(
        order_id=order_id,
        occurred_at=occurred_at,
        payload={
            "type": "EscrowFunded",
            "escrow_id": escrow_id,
            "create_tx_hash": create_tx_hash,
        },
    )


def escrow_released(
    order_id: str,
    escrow_id: str | None,
    release_tx_hash: str | None,
    occurred_at: datetime,
) -> OrderDomainEvent:
    return OrderDomainEvent(
        order_id=order_id,
        occurred_at=occurred_at,
        payload={
            "type": "EscrowReleased",
            "escrow_id": escrow_id,
            "release_tx_hash": release_tx_hash,
        },
    )


def escrow_refunded(
    order_id: str,
    escrow_id: str | None,
    refund_tx_hash: str | None,
    occurred_at: datetime,
) -> OrderDomainEvent:
    return OrderDomainEvent(
        order_id=order_id,
        occurred_at=occurred_at,
        payload={
            "type": "EscrowRefunded",
            "escrow_id": escrow_id,
            "refund_tx_hash": refund_tx_hash,
        },
    )


def dispute_opened(order_id: str, occurred_at: datetime) -> OrderDomainEvent:
    return OrderDomainEvent(
        order_id=order_id,
        occurred_at=occurred_at,
        payload={"type": "DisputeOpened"},
    )


def dispute_resolved(
    order_id: str,
    resolution: str,
    occurred_at: datetime,
    **kwargs: Any,
) -> OrderDomainEvent:
    return OrderDomainEvent(
        order_id=order_id,
        occurred_at=occurred_at,
        payload={"type": "DisputeResolved", "resolution": resolution, **kwargs},
    )
