"""
Circuit breaker + exponential-backoff retry for external HTTP calls.
Usage:
    cb = CircuitBreaker("binance")
    result = await cb.call(my_async_fn, arg1, arg2)
"""

import asyncio
import logging
import time
from typing import Any, Callable, Coroutine

logger = logging.getLogger("rsc-backend")


class CircuitOpenError(Exception):
    """Raised when the circuit is open and calls are blocked."""
    pass


class CircuitBreaker:
    """
    Simple async circuit breaker with three states: CLOSED, OPEN, HALF_OPEN.
    - CLOSED: requests flow normally; failures increment counter.
    - OPEN: requests are rejected immediately for `recovery_timeout` seconds.
    - HALF_OPEN: one probe request allowed; success resets, failure reopens.
    """

    def __init__(
        self,
        name: str,
        failure_threshold: int = 5,
        recovery_timeout: float = 30.0,
        half_open_max_calls: int = 1,
    ):
        self.name = name
        self.failure_threshold = failure_threshold
        self.recovery_timeout = recovery_timeout
        self.half_open_max_calls = half_open_max_calls

        self._state = "CLOSED"
        self._failure_count = 0
        self._last_failure_time: float = 0
        self._half_open_calls = 0

    @property
    def state(self) -> str:
        if self._state == "OPEN":
            if time.monotonic() - self._last_failure_time >= self.recovery_timeout:
                self._state = "HALF_OPEN"
                self._half_open_calls = 0
        return self._state

    def _record_success(self) -> None:
        self._failure_count = 0
        self._state = "CLOSED"
        self._half_open_calls = 0

    def _record_failure(self) -> None:
        self._failure_count += 1
        self._last_failure_time = time.monotonic()
        if self._failure_count >= self.failure_threshold or self._state == "HALF_OPEN":
            self._state = "OPEN"
            logger.warning("circuit_breaker name=%s state=OPEN after %d failures", self.name, self._failure_count)

    async def call(self, fn: Callable[..., Coroutine], *args: Any, **kwargs: Any) -> Any:
        current = self.state
        if current == "OPEN":
            raise CircuitOpenError(f"Circuit '{self.name}' is OPEN — call rejected")
        if current == "HALF_OPEN":
            self._half_open_calls += 1
            if self._half_open_calls > self.half_open_max_calls:
                raise CircuitOpenError(f"Circuit '{self.name}' is HALF_OPEN — max probe calls reached")
        try:
            result = await fn(*args, **kwargs)
            self._record_success()
            return result
        except Exception:
            self._record_failure()
            raise


async def retry_with_backoff(
    fn: Callable[..., Coroutine],
    *args: Any,
    max_retries: int = 3,
    base_delay: float = 0.5,
    max_delay: float = 8.0,
    retryable_exceptions: tuple = (Exception,),
    **kwargs: Any,
) -> Any:
    """
    Retry an async function with exponential backoff.
    Returns the result on success, raises the last exception on exhaustion.
    """
    last_exc: Exception | None = None
    for attempt in range(max_retries + 1):
        try:
            return await fn(*args, **kwargs)
        except retryable_exceptions as exc:
            last_exc = exc
            if attempt < max_retries:
                delay = min(base_delay * (2 ** attempt), max_delay)
                logger.debug("retry attempt=%d/%d delay=%.1fs error=%s", attempt + 1, max_retries, delay, exc)
                await asyncio.sleep(delay)
    raise last_exc  # type: ignore[misc]
