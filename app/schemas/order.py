"""
Schemas de órdenes (alineados con types/index.ts del frontend).
"""

from typing import Literal

from pydantic import BaseModel, ConfigDict, Field

OrderStatus = Literal[
    "CREATED",
    "AWAITING_PAYMENT",
    "ESCROW_FUNDED",
    "RELEASED",
    "CANCELLED",
    "DISPUTED",
]
CancelledBy = Literal["SELLER", "BUYER"] | None


class OrderSeller(BaseModel):
    id: str
    wallet_address: str
    reputation_score: float = 0


class OrderBuyer(BaseModel):
    id: str
    wallet_address: str
    reputation_score: float = 0


class Order(BaseModel):
    id: str
    sellerId: str
    buyerId: str | None = None
    seller: OrderSeller | None = None
    buyer: OrderBuyer | None = None
    cryptoCurrency: str
    cryptoAmount: str
    fiatCurrency: str
    fiatAmount: str
    pricePerUnit: str | None = None
    status: OrderStatus
    escrowId: str | None = None
    paymentMethod: str | None = None
    terms: str | None = None
    expiresAt: str | None = None
    acceptedAt: str | None = None
    completedAt: str | None = None
    cancelledAt: str | None = None
    cancelledBy: CancelledBy = None
    disputedAt: str | None = None
    createdAt: str
    updatedAt: str


class OrderListParams(BaseModel):
    page: int = Field(1, ge=1, description="Page number")
    limit: int = Field(50, ge=1, le=100, description="Items per page")
    status: OrderStatus | None = None
    sellerId: str | None = None
    buyerId: str | None = None
    cryptoCurrency: str | None = None
    fiatCurrency: str | None = None


class CreateOrderBody(BaseModel):
    model_config = ConfigDict(strict=True, str_strip_whitespace=True)

    cryptoCurrency: str = Field(min_length=2, max_length=20)
    cryptoAmount: str = Field(min_length=1, max_length=64)
    fiatCurrency: str = Field(min_length=2, max_length=20)
    fiatAmount: str = Field(min_length=1, max_length=64)
    pricePerUnit: str = Field(min_length=1, max_length=64)
    paymentMethod: str = Field(min_length=2, max_length=120)
    terms: str | None = Field(default=None, max_length=500)
    expiresAt: str | None = None
    chainId: int | None = None
    tokenAddress: str | None = None
    blockchain: str | None = None
    escrowTxHash: str | None = None
    escrowContractAddress: str | None = None


class AcceptOrderBody(BaseModel):
    paymentMethod: str | None = Field(default=None, max_length=120)
