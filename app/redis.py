"""
Redis async client singleton.
Provides cache, distributed rate limiting, and Socket.IO adapter support.
Falls back gracefully when Redis is unavailable (development without Redis).
"""

import logging
from typing import Optional

import redis.asyncio as aioredis

from app.config import settings

logger = logging.getLogger("rsc-backend")

_redis_client: Optional[aioredis.Redis] = None
_redis_available: bool = False


async def get_redis() -> Optional[aioredis.Redis]:
    """Return the shared Redis client, or None if unavailable."""
    global _redis_client, _redis_available
    if _redis_client is not None:
        return _redis_client if _redis_available else None
    try:
        _redis_client = aioredis.from_url(
            settings.redis_url,
            decode_responses=True,
            socket_connect_timeout=3,
            retry_on_timeout=True,
        )
        await _redis_client.ping()
        _redis_available = True
        logger.info("Redis connected: %s", settings.redis_url)
        return _redis_client
    except Exception as exc:
        logger.warning("Redis unavailable (%s), falling back to in-memory: %s", settings.redis_url, exc)
        _redis_available = False
        return None


async def close_redis() -> None:
    global _redis_client, _redis_available
    if _redis_client is not None:
        try:
            await _redis_client.aclose()
        except Exception:
            pass
        _redis_client = None
        _redis_available = False


async def redis_rate_limit(key: str, window: int, max_requests: int) -> bool:
    """
    Distributed sliding-window rate limit via Redis sorted set.
    Returns True if the request is allowed, False if rate-limited.
    Returns True (allow) if Redis is unavailable.
    """
    client = await get_redis()
    if client is None:
        return True
    import time
    now = time.time()
    pipe = client.pipeline()
    pipe.zremrangebyscore(key, 0, now - window)
    pipe.zadd(key, {f"{now}": now})
    pipe.zcard(key)
    pipe.expire(key, window + 1)
    results = await pipe.execute()
    count = results[2]
    return count <= max_requests
