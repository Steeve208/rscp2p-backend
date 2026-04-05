"""
Mercado: GET /api/terminal/market/prices (OHLC para gráficos).
"""

from fastapi import APIRouter, Query

from app.schemas.market import OHLCCandle
from app.services.market import get_prices

router = APIRouter(prefix="/terminal/market", tags=["terminal-market"])


@router.get("/prices", response_model=list[OHLCCandle])
async def market_prices(
    symbol: str = Query("BTCUSDT", description="Par de trading"),
    timeframe: str = Query("1h", description="1m, 5m, 15m, 1h, 4h, 1d"),
    limit: int = Query(100, ge=1, le=500),
):
    return await get_prices(symbol=symbol, timeframe=timeframe, limit=limit)
