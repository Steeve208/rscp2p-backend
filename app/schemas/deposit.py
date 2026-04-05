"""
Schemas para el endpoint de depósitos (blockchain watcher webhook).
"""

from pydantic import BaseModel, Field


class DepositEventBody(BaseModel):
    orderId: str = Field(min_length=1)
    txHash: str = Field(min_length=1, max_length=66)
    amount: str = Field(min_length=1, max_length=64)
    currency: str = Field(min_length=1, max_length=20)
    externalEscrowId: str = Field(min_length=1)
    contractAddress: str = Field(min_length=1, max_length=66)
    idempotencyKey: str | None = None


class DepositResultResponse(BaseModel):
    orderId: str
    orderStatus: str
    escrowFunded: bool
    alreadyProcessed: bool
    rejected: bool = False
    rejectionReason: str | None = None
