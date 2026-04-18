"""
Seed inicial para Launchpad: gems, presale, audit, token info, etc.
Se ejecuta al arrancar el backend si la DB está vacía (producción).
"""

from datetime import datetime, timezone
from uuid import uuid4

from sqlalchemy import select
from sqlalchemy.orm import Session

from app.db import Base, SessionLocal, engine, ensure_launchpad_presale_status_column
from app.launchpad.models import (
    AuditModel,
    GemModel,
    OrderBookEntryModel,
    PresaleModel,
    PriceHistoryModel,
    TokenInfoModel,
    TokenSentimentModel,
)


def _iso_now():
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def run_seed(db: Session) -> None:
    if db.execute(select(GemModel)).scalars().first() is not None:
        return

    g1 = GemModel(
        id=str(uuid4()),
        project_icon="🛡️",
        project_name="AETHER PROTOCOL",
        description="DeFi Cross-chain Bridging",
        security_score=99,
        price_change=12.4,
        liquidity_depth="$2.4M",
        upvotes="14204",
        launch_date=_iso_now(),
        sparkline_data=[0.8, 0.85, 0.82, 0.88, 0.9, 0.87, 0.92],
        contract_address="0xaether12345678901234567890123456789012",
        category="DeFi",
        is_verified=True,
        rug_checked=True,
        price=0.85,
        volume_24h=2400000,
        is_featured=True,
    )
    db.add(g1)
    db.flush()

    p1 = PresaleModel(
        id=str(uuid4()),
        gem_id=g1.id,
        contract_address="0xaether12345678901234567890123456789012",
        project_name="AETHER PROTOCOL",
        project_description="DeFi Cross-chain Bridging for institutional grade compliance.",
        project_icon="🛡️",
        is_verified=True,
        token_symbol="AETH",
        exchange_rate=25000,
        min_buy="0.1",
        max_buy="10.0",
        end_date=_iso_now(),
        soft_cap="300",
        hard_cap="500",
        min_contrib="0.1",
        max_contrib="10",
        vesting_tge="25%",
        vesting_cliff="1 Month",
        vesting_linear="6 Months",
        vesting_total_months=7,
        audit_url="https://audit.example.com",
        contract_url="https://etherscan.io/address/0xaether12345678901234567890123456789012",
    )
    db.add(p1)

    ti1 = TokenInfoModel(
        id=str(uuid4()),
        gem_id=g1.id,
        contract_address="0xaether12345678901234567890123456789012",
        total_supply="100M",
        burned="12.5%",
        dev_wallet_lock_days=245,
    )
    db.add(ti1)
    db.flush()

    ts1 = TokenSentimentModel(
        id=str(uuid4()),
        token_info_id=ti1.id,
        score=99,
        label="Extremely Bullish",
        comments=[{"author": "0xWhale...f2", "timestamp": "2m ago", "text": "Security score 99. Solid fundamentals."}],
        bullish_count=92,
        bearish_count=8,
    )
    db.add(ts1)

    for i, price in enumerate([0.8, 0.85, 0.82, 0.88, 0.9, 0.87, 0.92]):
        db.add(
            PriceHistoryModel(
                id=str(uuid4()),
                contract_address="0xaether12345678901234567890123456789012",
                time=f"0{i}:00",
                price=price,
                volume=price * 15000,
            )
        )

    for side, mult in [("sell", 1.003), ("sell", 1.004), ("buy", 0.998), ("buy", 0.997)]:
        db.add(
            OrderBookEntryModel(
                id=str(uuid4()),
                contract_address="0xaether12345678901234567890123456789012",
                side=side,
                price=f"{(0.85 * mult):.4f}",
                amount="12500",
                score=98.0,
                address="0x1234567890123456789012345678901234567890",
            )
        )

    a1 = AuditModel(
        id=str(uuid4()),
        contract_address="0xaether12345678901234567890123456789012",
        project_icon="🛡️",
        project_name="AETHER Protocol",
        full_address="0xaether12345678901234567890123456789012",
        network="Ethereum Mainnet",
        audit_completed="24h ago",
        is_verified=True,
        verdict="RUG-PROOF VERIFIED",
        risk_level="VERY LOW",
        trust_score=98,
        trust_summary="Highly secure contract patterns with long-term liquidity commitment.",
        security_checks=[
            {"name": "Ownership Renounced", "status": "PASSED", "description": "Contract creator cannot modify logic or mint tokens.", "tooltip": "Ownership renounced."},
            {"name": "No Proxy Detected", "status": "PASSED", "description": "Logic cannot be upgraded via proxy.", "tooltip": "Immutable."},
        ],
        vulnerabilities={"critical": 0, "high": 0, "medium": 2, "low": 5},
        liquidity_locks={
            "totalLocked": "99.2% LOCKED",
            "locks": [
                {"lockerName": "Unicrypt Locker", "contractAddress": "0x663d...90a1", "amount": "450.5 ETH LP", "unlocksIn": "364 Days", "unlockDate": "Nov 12, 2025", "txHash": "0x1234...abcd"},
            ],
        },
        community_sentiment={"bullish": 92, "bearish": 8, "upvotes": "14.2k", "watchlists": "1.2k", "comments": [{"author": "CryptoWhale77", "reputation": "Expert", "text": "Locked LP looks solid."}]},
        token_symbol="AETH",
    )
    db.add(a1)

    g2 = GemModel(
        id=str(uuid4()),
        project_icon="📦",
        project_name="VOX-NET",
        description="Decentralized Storage",
        security_score=92,
        price_change=4.8,
        liquidity_depth="$1.1M",
        upvotes="8112",
        launch_date=_iso_now(),
        sparkline_data=[0.5, 0.52, 0.51, 0.53, 0.54, 0.52, 0.55],
        contract_address="0xvox1234567890123456789012345678901234",
        category="Privacy",
        is_verified=True,
        rug_checked=True,
        price=0.52,
        volume_24h=1100000,
        is_featured=False,
    )
    db.add(g2)

    db.commit()


def init_db() -> None:
    from app.launchpad import models  # noqa: F401
    from app.models import marketplace  # noqa: F401 - orders, escrows, disputes, users

    Base.metadata.create_all(bind=engine)
    ensure_launchpad_presale_status_column()
    db = SessionLocal()
    try:
        run_seed(db)
    finally:
        db.close()
