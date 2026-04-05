"""
Escrow as Value Object within the Order aggregate.
Escrow has no identity outside Order; all state changes go through Order aggregate.
Invariants: amount must equal Order amount; status must be consistent with Order status.
"""

from dataclasses import dataclass
from datetime import datetime
from typing import Optional


# Escrow status values (must match Order aggregate invariants)
ESCROW_PENDING = "PENDING"
ESCROW_FUNDED = "FUNDED"
ESCROW_RELEASED = "RELEASED"
ESCROW_REFUNDED = "REFUNDED"

ESCROW_STATES = frozenset({ESCROW_PENDING, ESCROW_FUNDED, ESCROW_RELEASED, ESCROW_REFUNDED})

# Legal escrow transitions: (from_state, to_state)
ESCROW_TRANSITIONS = frozenset({
    (ESCROW_PENDING, ESCROW_FUNDED),
    (ESCROW_FUNDED, ESCROW_RELEASED),
    (ESCROW_FUNDED, ESCROW_REFUNDED),
})


@dataclass(frozen=True)
class EscrowValueObject:
    """
    Immutable value object representing escrow state within an Order.
    Built from persistence (EscrowModel) or from deposit data; never mutated directly.
    """

    id: str
    order_id: str
    external_escrow_id: str
    contract_address: str
    crypto_amount: str
    crypto_currency: str
    status: str
    create_tx_hash: Optional[str] = None
    release_tx_hash: Optional[str] = None
    refund_tx_hash: Optional[str] = None
    locked_at: Optional[datetime] = None
    released_at: Optional[datetime] = None
    refunded_at: Optional[datetime] = None

    def __post_init__(self) -> None:
        if self.status not in ESCROW_STATES:
            raise ValueError(f"Invalid escrow status: {self.status}")

    def can_transition_to(self, new_status: str) -> bool:
        return (self.status, new_status) in ESCROW_TRANSITIONS
