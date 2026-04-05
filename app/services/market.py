"""
Servicio de precios/OHLC para gráficos.
Integra Binance public API con cache Redis + circuit breaker.
"""

import json
import logging

import httpx

from app.redis import get_redis
from app.schemas.market import OHLCCandle
from app.utils.resilience import CircuitBreaker, CircuitOpenError, retry_with_backoff

logger = logging.getLogger("rsc-backend")

BINANCE_KLINES_URL = "https://api.binance.com/api/v3/klines"

_binance_circuit = CircuitBreaker("binance", failure_threshold=5, recovery_timeout=60.0)

TIMEFRAME_MAP = {
    "1m": ("1m", 10),
    "5m": ("5m", 30),
    "15m": ("15m", 60),
    "1h": ("1h", 120),
    "4h": ("4h", 300),
    "1d": ("1d", 600),
}


async def get_prices(
    symbol: str = "BTCUSDT",
    timeframe: str = "1h",
    limit: int = 100,
) -> list[OHLCCandle]:
    """Fetch OHLC candles from Binance with Redis caching."""
    interval, cache_ttl = TIMEFRAME_MAP.get(timeframe, ("1h", 120))
    cache_key = f"ohlc:{symbol}:{interval}:{limit}"

    redis = await get_redis()
    if redis:
        try:
            cached = await redis.get(cache_key)
            if cached:
                return [OHLCCandle(**c) for c in json.loads(cached)]
        except Exception:
            pass

    async def _fetch_binance() -> list:
        async with httpx.AsyncClient(timeout=10) as client:
            resp = await client.get(
                BINANCE_KLINES_URL,
                params={"symbol": symbol, "interval": interval, "limit": limit},
            )
            resp.raise_for_status()
            return resp.json()

    try:
        raw = await _binance_circuit.call(
            retry_with_backoff,
            _fetch_binance,
            max_retries=2,
            base_delay=1.0,
            retryable_exceptions=(httpx.HTTPError, httpx.TimeoutException, OSError),
        )
    except CircuitOpenError:
        logger.warning("Binance circuit OPEN for %s/%s — returning empty", symbol, interval)
        return []
    except Exception as exc:
        logger.warning("Binance API error for %s/%s: %s", symbol, interval, exc)
        return []

    candles = []
    for k in raw:
        candles.append(OHLCCandle(
            time=int(k[0]) // 1000,
            open=float(k[1]),
            high=float(k[2]),
            low=float(k[3]),
            close=float(k[4]),
            volume=float(k[5]),
        ))

    if redis and candles:
        try:
            await redis.setex(cache_key, cache_ttl, json.dumps([c.model_dump() for c in candles]))
        except Exception:
            pass

    return candles
