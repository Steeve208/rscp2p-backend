"""
Schemas de escrow para Marketplace.
"""

from typing import Literal

from pydantic import BaseModel, Field

EscrowStatus = Literal["PENDING", "FUNDED", "RELEASED", "REFUNDED"]


class Escrow(BaseModel):
    id: str
    orderId: str
    escrowId: str
    contractAddress: str
    cryptoAmount: str
    cryptoCurrency: str
    status: EscrowStatus = "PENDING"
    createTransactionHash: str | None = None
    releaseTransactionHash: str | None = None
    refundTransactionHash: str | None = None
    lockedAt: str | None = None
    releasedAt: str | None = None
    refundedAt: str | None = None
    createdAt: str
    updatedAt: str


class CreateEscrowBody(BaseModel):
    orderId: str = Field(min_length=1)
    escrowId: str = Field(min_length=1)
    contractAddress: str = Field(min_length=1, max_length=255)
    cryptoAmount: str = Field(min_length=1, max_length=64)
    cryptoCurrency: str = Field(min_length=2, max_length=20)
    createTransactionHash: str | None = Field(default=None, max_length=255)


class UpdateEscrowBody(BaseModel):
    status: EscrowStatus | None = None
    createTransactionHash: str | None = Field(default=None, max_length=255)
    releaseTransactionHash: str | None = Field(default=None, max_length=255)
    refundTransactionHash: str | None = Field(default=None, max_length=255)
    releasedAt: str | None = None
    refundedAt: str | None = None
