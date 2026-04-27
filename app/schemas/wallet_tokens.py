from __future__ import annotations

from pydantic import BaseModel, Field


class WalletTokenItem(BaseModel):
    chainId: int = Field(..., description="EVM chain id (e.g. 1, 56, 137, 42161, 8453)")
    address: str = Field(..., description="Token contract address (0x...) or 0x0 for native")
    symbol: str
    name: str
    decimals: int
    balance: str = Field(..., description="Human-readable balance (decimal string)")
    balanceRaw: str = Field(..., description="Raw integer balance as string")
    priceUsd: float | None = Field(None, description="USD price from CoinGecko if available")


class WalletTokensResponse(BaseModel):
    walletAddress: str
    chainId: int
    tokens: list[WalletTokenItem]

