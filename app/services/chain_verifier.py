"""
On-chain transaction verification via JSON-RPC.
Verifies that a tx_hash exists, is confirmed with enough blocks, and matches expected parameters.
Skips verification gracefully when RPC_URL is not configured (development).
"""

import logging

import httpx

from app.config import settings
from app.utils.resilience import CircuitBreaker, retry_with_backoff

logger = logging.getLogger("rsc-backend")

_rpc_circuit = CircuitBreaker("rpc", failure_threshold=5, recovery_timeout=30.0)


class ChainVerificationError(Exception):
    """Raised when on-chain verification fails."""
    pass


async def _raw_rpc_call(method: str, params: list) -> dict:
    """Execute a single JSON-RPC call against the configured node."""
    async with httpx.AsyncClient(timeout=15) as client:
        resp = await client.post(
            settings.rpc_url,
            json={"jsonrpc": "2.0", "id": 1, "method": method, "params": params},
        )
        resp.raise_for_status()
        data = resp.json()
        if "error" in data and data["error"]:
            raise ChainVerificationError(f"RPC error: {data['error']}")
        return data.get("result", {})


async def _rpc_call(method: str, params: list) -> dict:
    """JSON-RPC call wrapped with circuit breaker + retry."""
    return await _rpc_circuit.call(
        retry_with_backoff,
        _raw_rpc_call, method, params,
        max_retries=2,
        base_delay=1.0,
        retryable_exceptions=(httpx.HTTPError, httpx.TimeoutException, OSError),
    )


async def verify_transaction(
    tx_hash: str,
    expected_contract: str | None = None,
    expected_amount: str | None = None,
    expected_currency: str | None = None,
) -> dict:
    """
    Verify a transaction on-chain. Returns tx details dict.
    Raises ChainVerificationError if verification fails.
    Returns empty dict (skip) if RPC_URL not configured.
    """
    if not settings.rpc_url:
        logger.debug("RPC_URL not configured, skipping on-chain verification for tx=%s", tx_hash)
        return {}

    tx = await _rpc_call("eth_getTransactionByHash", [tx_hash])
    if not tx:
        raise ChainVerificationError(f"Transaction not found: {tx_hash}")

    receipt = await _rpc_call("eth_getTransactionReceipt", [tx_hash])
    if not receipt:
        raise ChainVerificationError(f"Transaction receipt not found: {tx_hash}")

    status = receipt.get("status")
    if status != "0x1":
        raise ChainVerificationError(f"Transaction reverted: {tx_hash}")

    block_hex = receipt.get("blockNumber")
    if not block_hex:
        raise ChainVerificationError(f"Transaction not mined: {tx_hash}")
    tx_block = int(block_hex, 16)

    latest_block_hex = await _rpc_call("eth_blockNumber", [])
    if isinstance(latest_block_hex, str):
        latest_block = int(latest_block_hex, 16)
    else:
        latest_block = 0

    confirmations = latest_block - tx_block
    if confirmations < settings.min_confirmations:
        raise ChainVerificationError(
            f"Insufficient confirmations: {confirmations}/{settings.min_confirmations} for tx={tx_hash}"
        )

    if expected_contract:
        tx_to = (tx.get("to") or "").lower()
        if tx_to != expected_contract.lower():
            raise ChainVerificationError(
                f"Contract mismatch: expected={expected_contract}, got={tx_to}"
            )

    chain_id_hex = tx.get("chainId")
    if chain_id_hex and settings.chain_id:
        tx_chain = int(chain_id_hex, 16)
        if tx_chain != settings.chain_id:
            raise ChainVerificationError(
                f"Chain ID mismatch: expected={settings.chain_id}, got={tx_chain}"
            )

    return {
        "tx_hash": tx_hash,
        "block_number": tx_block,
        "confirmations": confirmations,
        "status": "confirmed",
        "to": tx.get("to"),
        "from": tx.get("from"),
        "value": tx.get("value"),
    }
