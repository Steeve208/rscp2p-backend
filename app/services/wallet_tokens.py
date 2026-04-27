from __future__ import annotations

import logging
from decimal import Decimal, InvalidOperation

import httpx

from app.config import settings

logger = logging.getLogger("rsc-backend")


# CoinGecko "platform" ids for EVM chains used by /simple/token_price/{platform}
COINGECKO_PLATFORM_BY_CHAIN_ID: dict[int, str] = {
    1: "ethereum",
    56: "binance-smart-chain",
    137: "polygon-pos",
    42161: "arbitrum-one",
    8453: "base",
}


def _to_decimal_str(value: str, decimals: int) -> str:
    try:
        d = Decimal(value) / (Decimal(10) ** Decimal(decimals))
        # Keep a reasonable precision but avoid scientific notation
        s = format(d.normalize(), "f")
        # normalize() can produce "" for 0 in some edge cases, guard it
        return s if s else "0"
    except (InvalidOperation, ZeroDivisionError):
        return "0"


async def _fetch_covalent_balances(chain_id: int, wallet_address: str) -> list[dict]:
    if not settings.covalent_api_key:
        raise RuntimeError("COVALENT_API_KEY is not configured")

    url = f"https://api.covalenthq.com/v1/{chain_id}/address/{wallet_address}/balances_v2/"
    params = {"key": settings.covalent_api_key, "nft": "false", "no-nft-fetch": "true"}
    async with httpx.AsyncClient(timeout=20) as client:
        resp = await client.get(url, params=params)
        resp.raise_for_status()
        data = resp.json()

    items = (((data or {}).get("data") or {}).get("items")) or []
    return items if isinstance(items, list) else []


async def _fetch_coingecko_prices(chain_id: int, contract_addresses: list[str]) -> dict[str, float]:
    platform = COINGECKO_PLATFORM_BY_CHAIN_ID.get(chain_id)
    if not platform or not contract_addresses:
        return {}

    # CoinGecko allows batching via comma-separated list; keep chunks small.
    # Using contract addresses lowercased for stable mapping.
    out: dict[str, float] = {}
    chunk_size = 80
    async with httpx.AsyncClient(timeout=20) as client:
        for i in range(0, len(contract_addresses), chunk_size):
            chunk = contract_addresses[i : i + chunk_size]
            q = ",".join(chunk)
            resp = await client.get(
                f"https://api.coingecko.com/api/v3/simple/token_price/{platform}",
                params={
                    "contract_addresses": q,
                    "vs_currencies": "usd",
                },
                headers={"Accept": "application/json"},
            )
            if resp.status_code != 200:
                logger.warning("CoinGecko token_price %s failed: HTTP %s", platform, resp.status_code)
                continue
            payload = resp.json() or {}
            if not isinstance(payload, dict):
                continue
            for addr, v in payload.items():
                try:
                    usd = float((v or {}).get("usd"))
                except Exception:
                    continue
                out[str(addr).lower()] = usd
    return out


async def get_wallet_tokens_with_prices(
    *,
    chain_id: int,
    wallet_address: str,
    min_balance_raw: int = 1,
    require_price: bool = True,
) -> list[dict]:
    """
    Returns tokens held by wallet (from Covalent) enriched with CoinGecko USD prices (when possible).
    Filters:
      - min_balance_raw: drop zero balances
      - require_price: when True, drop tokens without CoinGecko USD price (except native if priced by covalent)
    """
    items = await _fetch_covalent_balances(chain_id, wallet_address)

    # Build a list of ERC20 addresses for CoinGecko (exclude native, exclude null addresses)
    erc20_addresses: list[str] = []
    normalized: list[dict] = []

    for it in items:
        try:
            bal_raw = str(it.get("balance") or "0")
            # some covalent items can have None decimals/symbols; be defensive
            decimals = int(it.get("contract_decimals") or 0)
            symbol = str(it.get("contract_ticker_symbol") or "").strip()
            name = str(it.get("contract_name") or "").strip()
            addr = str(it.get("contract_address") or "").strip().lower()
            is_native = bool(it.get("native_token"))
        except Exception:
            continue

        # Filter zero balances
        try:
            if int(bal_raw) < int(min_balance_raw):
                continue
        except Exception:
            continue

        # Native token: covalent includes quote sometimes; keep it.
        if is_native:
            price_usd = None
            try:
                q = it.get("quote_rate")
                price_usd = float(q) if q is not None else None
            except Exception:
                price_usd = None

            normalized.append(
                {
                    "chainId": chain_id,
                    "address": "0x0000000000000000000000000000000000000000",
                    "symbol": symbol or "NATIVE",
                    "name": name or "Native Token",
                    "decimals": decimals or 18,
                    "balanceRaw": bal_raw,
                    "balance": _to_decimal_str(bal_raw, decimals or 18),
                    "priceUsd": price_usd,
                    "_isNative": True,
                }
            )
            continue

        # ERC20
        if addr and addr.startswith("0x") and len(addr) == 42:
            erc20_addresses.append(addr)
        normalized.append(
            {
                "chainId": chain_id,
                "address": addr or "0x0000000000000000000000000000000000000000",
                "symbol": symbol or "TOKEN",
                "name": name or "Token",
                "decimals": decimals or 18,
                "balanceRaw": bal_raw,
                "balance": _to_decimal_str(bal_raw, decimals or 18),
                "priceUsd": None,
                "_isNative": False,
            }
        )

    prices = await _fetch_coingecko_prices(chain_id, sorted(set(erc20_addresses)))

    out: list[dict] = []
    for t in normalized:
        if not t.get("_isNative"):
            addr = str(t.get("address") or "").lower()
            t["priceUsd"] = prices.get(addr)
        # Apply price filter if requested
        if require_price:
            if t.get("priceUsd") is None:
                continue
        t.pop("_isNative", None)
        out.append(t)

    # Sort by USD value desc (balance * price) when available, else by balance
    def _sort_key(x: dict):
        try:
            bal = Decimal(str(x.get("balance") or "0"))
        except Exception:
            bal = Decimal(0)
        p = x.get("priceUsd")
        try:
            return float(bal) * float(p or 0)
        except Exception:
            return float(bal)

    out.sort(key=_sort_key, reverse=True)
    return out

