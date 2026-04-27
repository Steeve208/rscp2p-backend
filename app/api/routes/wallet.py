from __future__ import annotations

from fastapi import APIRouter, Query

from app.schemas.wallet_tokens import WalletTokensResponse, WalletTokenItem
from app.services.wallet_tokens import get_wallet_tokens_with_prices

router = APIRouter(prefix="/wallet", tags=["wallet"])


@router.get("/tokens", response_model=WalletTokensResponse)
async def wallet_tokens(
    walletAddress: str = Query(..., description="0x... wallet address"),
    chainId: int = Query(..., description="EVM chain id"),
    requirePrice: bool = Query(True, description="When true, only return tokens with USD price"),
):
    tokens_raw = await get_wallet_tokens_with_prices(
        chain_id=chainId,
        wallet_address=walletAddress,
        require_price=requirePrice,
    )
    tokens = [WalletTokenItem(**t) for t in tokens_raw]
    return WalletTokensResponse(walletAddress=walletAddress, chainId=chainId, tokens=tokens)

