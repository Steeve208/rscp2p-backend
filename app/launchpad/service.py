"""
Servicios Launchpad: CRUD y lógica de negocio.
Sin mocks: toda la información proviene de la base de datos.
"""

from datetime import datetime, timezone
from typing import Sequence

from sqlalchemy import and_, desc, func, or_, select
from sqlalchemy.orm import Session

from app.launchpad.models import (
    AuditCommentModel,
    AuditModel,
    ContributionModel,
    GemModel,
    OrderBookEntryModel,
    PresaleModel,
    PriceHistoryModel,
    SentimentVoteModel,
    SubmissionModel,
    TokenInfoModel,
    TokenSentimentModel,
    WatchlistModel,
)
from app.launchpad.schemas import (
    AuditCommentBody,
    AuditResponse,
    ContributionResponse,
    DaoSentimentSchema,
    FeaturedGemResponse,
    GemResponse,
    GlobalStatsResponse,
    OrderBookOffer,
    OrderBookResponse,
    PresaleContributionItem,
    PresaleResponse,
    PostPresaleContributionBody,
    PriceHistoryPoint,
    SentimentComment,
    SentimentResponse,
    SentimentVoteBody,
    SubmissionPostBody,
    TokenDetailResponse,
    TokenomicsResponse,
    TokenomicsSchema,
    VestingTerms,
)


def _iso(dt: datetime | None) -> str:
    if dt is None:
        return ""
    return dt.isoformat().replace("+00:00", "Z")


# ----- Gems -----
def list_gems(
    db: Session,
    page: int = 1,
    limit: int = 20,
    category: str | None = None,
    search: str | None = None,
    verified: bool | None = None,
    rug_checked: bool | None = None,
    min_score: float | None = None,
    max_score: float | None = None,
) -> tuple[list[GemResponse], int]:
    q = select(GemModel)
    count_q = select(func.count()).select_from(GemModel)
    if category:
        q = q.where(GemModel.category == category)
        count_q = count_q.where(GemModel.category == category)
    if search and search.strip():
        s = f"%{search.strip()}%"
        q = q.where(
            or_(
                GemModel.project_name.ilike(s),
                GemModel.description.ilike(s),
                GemModel.contract_address.ilike(s),
                GemModel.category.ilike(s),
            )
        )
        count_q = count_q.where(
            or_(
                GemModel.project_name.ilike(s),
                GemModel.description.ilike(s),
                GemModel.contract_address.ilike(s),
                GemModel.category.ilike(s),
            )
        )
    if verified is not None:
        q = q.where(GemModel.is_verified == verified)
        count_q = count_q.where(GemModel.is_verified == verified)
    if rug_checked is not None:
        q = q.where(GemModel.rug_checked == rug_checked)
        count_q = count_q.where(GemModel.rug_checked == rug_checked)
    if min_score is not None:
        q = q.where(GemModel.security_score >= min_score)
        count_q = count_q.where(GemModel.security_score >= min_score)
    if max_score is not None:
        q = q.where(GemModel.security_score <= max_score)
        count_q = count_q.where(GemModel.security_score <= max_score)

    total = db.scalar(count_q) or 0
    q = q.order_by(desc(GemModel.created_at)).offset((page - 1) * limit).limit(limit)
    rows = db.execute(q).scalars().all()
    items = [
        GemResponse(
            projectIcon=r.project_icon or "⭐",
            projectName=r.project_name or "",
            description=r.description or "",
            securityScore=r.security_score or 0,
            priceChange=r.price_change or 0,
            liquidityDepth=r.liquidity_depth or "$0",
            upvotes=r.upvotes or "0",
            launchDate=r.launch_date or "",
            sparklineData=r.sparkline_data or [],
            contractAddress=r.contract_address,
            category=r.category,
            isVerified=r.is_verified,
            rugChecked=r.rug_checked,
            price=r.price,
            volume24h=r.volume_24h,
        )
        for r in rows
    ]
    return items, total


def get_gem_by_address(db: Session, address: str) -> GemResponse | None:
    row = db.execute(select(GemModel).where(GemModel.contract_address == address)).scalars().first()
    if not row:
        return None
    return GemResponse(
        projectIcon=row.project_icon or "⭐",
        projectName=row.project_name or "",
        description=row.description or "",
        securityScore=row.security_score or 0,
        priceChange=row.price_change or 0,
        liquidityDepth=row.liquidity_depth or "$0",
        upvotes=row.upvotes or "0",
        launchDate=row.launch_date or "",
        sparklineData=row.sparkline_data or [],
        contractAddress=row.contract_address,
        category=row.category,
        isVerified=row.is_verified,
        rugChecked=row.rug_checked,
        price=row.price,
        volume24h=row.volume_24h,
    )


def get_featured_gem(db: Session) -> FeaturedGemResponse | None:
    row = db.execute(select(GemModel).where(GemModel.is_featured == True).limit(1)).scalars().first()
    if not row:
        return None
    presale = db.execute(select(PresaleModel).where(PresaleModel.gem_id == row.id)).scalars().first()
    end_time = ""
    raised = None
    target = None
    if presale:
        end_time = presale.end_date or ""
        try:
            target = float(presale.hard_cap) if presale.hard_cap else None
        except (TypeError, ValueError):
            pass
        contrib_rows = db.execute(select(ContributionModel.amount).where(ContributionModel.presale_id == presale.id)).scalars().all()
        raised = 0.0
        for amt in contrib_rows:
            try:
                raised += float(amt)
            except (TypeError, ValueError):
                pass
    return FeaturedGemResponse(
        projectName=row.project_name or "",
        subtitle=row.category or "",
        description=row.description or "",
        endTime=end_time or _iso(row.created_at),
        contractAddress=row.contract_address,
        projectIcon=row.project_icon,
        category=row.category,
        raised=raised,
        target=target,
        participants=None,
        watchingCount=None,
        trendingRank=None,
    )


def get_global_stats(db: Session) -> GlobalStatsResponse:
    stats = db.execute(
        select(
            func.count(GemModel.id),
            func.coalesce(func.avg(GemModel.security_score), 0),
            func.coalesce(func.sum(GemModel.volume_24h), 0),
        ).select_from(GemModel)
    ).one()
    total_gems = stats[0] or 0
    avg_score = float(stats[1])
    total_vol = float(stats[2])

    total_liq = 0.0
    liq_values = db.scalars(select(GemModel.liquidity_depth)).all()
    for raw in liq_values:
        try:
            s = (raw or "").replace("$", "").replace("M", "").replace(",", "")
            if s:
                total_liq += float(s)
        except (TypeError, ValueError):
            pass

    active_presales = db.scalar(
        select(func.count()).select_from(PresaleModel).where(PresaleModel.status == "active")
    ) or 0

    return GlobalStatsResponse(
        totalGems=total_gems,
        totalLiquidity=f"${total_liq:.1f}M" if total_liq else "$0",
        avgSecurityScore=round(avg_score * 10) / 10,
        activePresales=active_presales,
        totalVolume24h=f"${total_vol / 1e6:.1f}M" if total_vol else "$0",
    )


# ----- Presales -----
def list_presales(
    db: Session,
    status: str | None = None,
    page: int = 1,
    limit: int = 20,
    search: str | None = None,
) -> tuple[list[PresaleResponse], int]:
    q = select(PresaleModel)
    count_q = select(func.count()).select_from(PresaleModel)
    if status:
        q = q.where(PresaleModel.status == status)
        count_q = count_q.where(PresaleModel.status == status)
    if search and search.strip():
        s = f"%{search.strip()}%"
        q = q.where(or_(PresaleModel.project_name.ilike(s), PresaleModel.token_symbol.ilike(s)))
        count_q = count_q.where(or_(PresaleModel.project_name.ilike(s), PresaleModel.token_symbol.ilike(s)))
    total = db.scalar(count_q) or 0
    q = q.order_by(desc(PresaleModel.created_at)).offset((page - 1) * limit).limit(limit)
    rows = db.execute(q).scalars().all()
    out = []
    for r in rows:
        out.append(_presale_to_response(r))
    return out, total


def _presale_to_response(r: PresaleModel) -> PresaleResponse:
    return PresaleResponse(
        id=r.id,
        projectName=r.project_name or "",
        projectDescription=r.project_description or "",
        projectIcon=r.project_icon or "⭐",
        isVerified=r.is_verified or False,
        contractAddress=r.contract_address,
        tokenSymbol=r.token_symbol or "",
        exchangeRate=r.exchange_rate or 0,
        minBuy=r.min_buy or "0",
        maxBuy=r.max_buy or "0",
        endDate=r.end_date or "",
        softCap=r.soft_cap or "0",
        hardCap=r.hard_cap or "0",
        minContrib=r.min_contrib or "0",
        maxContrib=r.max_contrib or "0",
        vestingTerms=VestingTerms(
            tgeUnlock=r.vesting_tge or "0%",
            cliffPeriod=r.vesting_cliff or "",
            linearVesting=r.vesting_linear or "",
            totalMonths=r.vesting_total_months,
        ),
        auditUrl=r.audit_url,
        contractUrl=r.contract_url,
        status=r.status or "active",
    )


def get_presale_by_id(db: Session, presale_id: str) -> PresaleResponse | None:
    r = db.execute(select(PresaleModel).where(PresaleModel.id == presale_id)).scalars().first()
    if not r:
        r = db.execute(select(PresaleModel).where(PresaleModel.contract_address == presale_id)).scalars().first()
    if not r:
        return None
    return _presale_to_response(r)


def _mask_wallet(address: str) -> str:
    """Show first 6 and last 4 characters, mask the rest."""
    if len(address) <= 10:
        return address
    return f"{address[:6]}...{address[-4:]}"


def get_presale_contributions(
    db: Session,
    presale_id: str,
    limit: int = 20,
    mask_wallets: bool = True,
) -> list[PresaleContributionItem]:
    presale = db.execute(select(PresaleModel).where(PresaleModel.id == presale_id)).scalars().first()
    if not presale:
        presale = db.execute(select(PresaleModel).where(PresaleModel.contract_address == presale_id)).scalars().first()
    if not presale:
        return []
    q = select(ContributionModel).where(ContributionModel.presale_id == presale.id).order_by(desc(ContributionModel.created_at)).limit(limit)
    rows = db.execute(q).scalars().all()
    return [
        PresaleContributionItem(
            id=c.id,
            walletAddress=_mask_wallet(c.wallet_address) if mask_wallets else c.wallet_address,
            amount=c.amount,
            timestamp=_iso(c.created_at),
        )
        for c in rows
    ]


def create_presale_contribution(
    db: Session,
    presale_id: str,
    body: PostPresaleContributionBody,
    wallet_address: str,
) -> ContributionResponse | None:
    """Create contribution. wallet_address must come from JWT (not body). Idempotent by (presale_id, tx_hash)."""
    presale = db.execute(select(PresaleModel).where(PresaleModel.id == presale_id)).scalars().first()
    if not presale:
        presale = db.execute(select(PresaleModel).where(PresaleModel.contract_address == presale_id)).scalars().first()
    if not presale:
        return None
    # Idempotency: same tx_hash for same presale returns existing contribution
    existing = db.execute(
        select(ContributionModel).where(
            ContributionModel.presale_id == presale.id,
            ContributionModel.tx_hash == body.txHash,
        ).limit(1)
    ).scalars().first()
    if existing:
        db.refresh(existing)
        presale = db.get(PresaleModel, existing.presale_id) or presale
        return ContributionResponse(
            id=existing.id,
            walletAddress=existing.wallet_address,
            projectName=presale.project_name or "",
            projectIcon=presale.project_icon or "⭐",
            tokenSymbol=presale.token_symbol or "",
            presaleId=presale.id,
            contribution=existing.amount,
            buyPrice=existing.buy_price or "0",
            currentValue=existing.current_value or "0",
            growth=existing.growth or "0",
            isLoss=existing.is_loss,
            vestingProgress=existing.vesting_progress or 0,
            nextUnlock=existing.next_unlock or "",
            claimableAmount=existing.claimable_amount,
            status=existing.status,
            txHash=existing.tx_hash,
            createdAt=_iso(existing.created_at),
        )
    c = ContributionModel(
        presale_id=presale.id,
        wallet_address=wallet_address,
        amount=body.amount,
        tx_hash=body.txHash,
        buy_price="0",
        current_value=body.amount,
        growth="0",
        is_loss=False,
        vesting_progress=0,
        next_unlock="",
        claimable_amount=None,
        status="active",
    )
    db.add(c)
    try:
        db.commit()
        db.refresh(c)
    except Exception as e:
        db.rollback()
        from sqlalchemy.exc import IntegrityError
        if not isinstance(e, IntegrityError):
            raise
        # Race: another request inserted same (presale_id, tx_hash); return existing
        existing = db.execute(
            select(ContributionModel).where(
                ContributionModel.presale_id == presale.id,
                ContributionModel.tx_hash == body.txHash,
            ).limit(1)
        ).scalars().first()
        if existing:
            presale = db.get(PresaleModel, existing.presale_id) or presale
            return ContributionResponse(
                id=existing.id,
                walletAddress=existing.wallet_address,
                projectName=presale.project_name or "",
                projectIcon=presale.project_icon or "⭐",
                tokenSymbol=presale.token_symbol or "",
                presaleId=presale.id,
                contribution=existing.amount,
                buyPrice=existing.buy_price or "0",
                currentValue=existing.current_value or "0",
                growth=existing.growth or "0",
                isLoss=existing.is_loss,
                vestingProgress=existing.vesting_progress or 0,
                nextUnlock=existing.next_unlock or "",
                claimableAmount=existing.claimable_amount,
                status=existing.status,
                txHash=existing.tx_hash,
                createdAt=_iso(existing.created_at),
            )
        raise
    return ContributionResponse(
        id=c.id,
        walletAddress=c.wallet_address,
        projectName=presale.project_name or "",
        projectIcon=presale.project_icon or "⭐",
        tokenSymbol=presale.token_symbol or "",
        presaleId=presale.id,
        contribution=c.amount,
        buyPrice=c.buy_price or "0",
        currentValue=c.current_value or "0",
        growth=c.growth or "0",
        isLoss=c.is_loss,
        vestingProgress=c.vesting_progress or 0,
        nextUnlock=c.next_unlock or "",
        claimableAmount=c.claimable_amount,
        status=c.status,
        txHash=c.tx_hash,
        createdAt=_iso(c.created_at),
    )


# ----- Contributions (portfolio) -----
def list_contributions_me(
    db: Session,
    wallet_address: str,
    status: str | None = None,
    search: str | None = None,
    page: int = 1,
    limit: int = 20,
) -> tuple[list[ContributionResponse], int]:
    q = select(ContributionModel).where(ContributionModel.wallet_address == wallet_address)
    count_q = select(func.count()).select_from(ContributionModel).where(ContributionModel.wallet_address == wallet_address)
    if status:
        q = q.where(ContributionModel.status == status)
        count_q = count_q.where(ContributionModel.status == status)
    if search and search.strip():
        s = f"%{search.strip()}%"
        sub = select(PresaleModel.id).where(or_(PresaleModel.project_name.ilike(s), PresaleModel.token_symbol.ilike(s)))
        q = q.where(ContributionModel.presale_id.in_(sub))
        count_q = count_q.where(ContributionModel.presale_id.in_(sub))
    total = db.scalar(count_q) or 0
    q = q.order_by(desc(ContributionModel.created_at)).offset((page - 1) * limit).limit(limit)
    rows = db.execute(q).scalars().all()
    out = []
    for c in rows:
        presale = db.get(PresaleModel, c.presale_id)
        out.append(
            ContributionResponse(
                id=c.id,
                walletAddress=c.wallet_address,
                projectName=presale.project_name if presale else "",
                projectIcon=presale.project_icon if presale else "⭐",
                tokenSymbol=presale.token_symbol if presale else "",
                presaleId=c.presale_id,
                contribution=c.amount,
                buyPrice=c.buy_price or "0",
                currentValue=c.current_value or "0",
                growth=c.growth or "0",
                isLoss=c.is_loss,
                vestingProgress=c.vesting_progress or 0,
                nextUnlock=c.next_unlock or "",
                claimableAmount=c.claimable_amount,
                status=c.status,
                txHash=c.tx_hash,
                createdAt=_iso(c.created_at),
            )
        )
    return out, total


def get_contribution_by_id(db: Session, contribution_id: str, wallet_address: str) -> ContributionResponse | None:
    c = db.execute(select(ContributionModel).where(ContributionModel.id == contribution_id, ContributionModel.wallet_address == wallet_address)).scalar_one_or_none()
    if not c:
        return None
    presale = db.get(PresaleModel, c.presale_id)
    return ContributionResponse(
        id=c.id,
        walletAddress=c.wallet_address,
        projectName=presale.project_name if presale else "",
        projectIcon=presale.project_icon if presale else "⭐",
        tokenSymbol=presale.token_symbol if presale else "",
        presaleId=c.presale_id,
        contribution=c.amount,
        buyPrice=c.buy_price or "0",
        currentValue=c.current_value or "0",
        growth=c.growth or "0",
        isLoss=c.is_loss,
        vestingProgress=c.vesting_progress or 0,
        nextUnlock=c.next_unlock or "",
        claimableAmount=c.claimable_amount,
        status=c.status,
        txHash=c.tx_hash,
        createdAt=_iso(c.created_at),
    )


def get_contribution_by_tx(db: Session, tx_hash: str, wallet_address: str) -> ContributionResponse | None:
    c = db.execute(select(ContributionModel).where(ContributionModel.tx_hash == tx_hash, ContributionModel.wallet_address == wallet_address)).scalars().first()
    if not c:
        return None
    presale = db.get(PresaleModel, c.presale_id)
    return ContributionResponse(
        id=c.id,
        walletAddress=c.wallet_address,
        projectName=presale.project_name if presale else "",
        projectIcon=presale.project_icon if presale else "⭐",
        tokenSymbol=presale.token_symbol if presale else "",
        presaleId=c.presale_id,
        contribution=c.amount,
        buyPrice=c.buy_price or "0",
        currentValue=c.current_value or "0",
        growth=c.growth or "0",
        isLoss=c.is_loss,
        vestingProgress=c.vesting_progress or 0,
        nextUnlock=c.next_unlock or "",
        claimableAmount=c.claimable_amount,
        status=c.status,
        txHash=c.tx_hash,
        createdAt=_iso(c.created_at),
    )


# ----- Tokens -----
def get_token_detail(db: Session, address: str) -> TokenDetailResponse | None:
    gem = db.execute(select(GemModel).where(GemModel.contract_address == address)).scalar_one_or_none()
    if not gem:
        return None
    token_info = db.execute(select(TokenInfoModel).where(TokenInfoModel.contract_address == address)).scalar_one_or_none()
    sentiment = None
    if token_info:
        sentiment = db.execute(select(TokenSentimentModel).where(TokenSentimentModel.token_info_id == token_info.id)).scalar_one_or_none()
    total_supply = "0"
    burned = "0%"
    dev_lock = 0
    if token_info:
        total_supply = token_info.total_supply or "0"
        burned = token_info.burned or "0%"
        dev_lock = token_info.dev_wallet_lock_days or 0
    score = sentiment.score if sentiment else (gem.security_score or 0)
    label = sentiment.label if sentiment else ("Bullish" if (gem.security_score or 0) >= 85 else "Neutral")
    comments = sentiment.comments if sentiment and isinstance(sentiment.comments, list) else []
    sentiment_comments = [SentimentComment(author=x.get("author", ""), timestamp=x.get("timestamp", ""), text=x.get("text", "")) for x in comments if isinstance(x, dict)]
    return TokenDetailResponse(
        projectIcon=gem.project_icon or "⭐",
        projectName=gem.project_name or "",
        symbol=(gem.project_name or "TOKEN")[:4].upper(),
        price=gem.price or 0,
        priceChange24h=gem.price_change or 0,
        isVerified=gem.is_verified or False,
        contractAddress=gem.contract_address,
        exchangeRate=1.0 / (gem.price or 1) if gem.price else 1000,
        sparklineData=gem.sparkline_data or [],
        tokenomics=TokenomicsSchema(totalSupply=total_supply, burned=burned, devWalletLockDays=dev_lock),
        daoSentiment=DaoSentimentSchema(score=score, label=label, comments=sentiment_comments),
    )


def get_tokenomics(db: Session, address: str) -> TokenomicsResponse | None:
    row = db.execute(select(TokenInfoModel).where(TokenInfoModel.contract_address == address)).scalars().first()
    if not row:
        gem = db.execute(select(GemModel).where(GemModel.contract_address == address)).scalars().first()
        if not gem:
            return None
        return TokenomicsResponse(totalSupply="0", burned="0%", devWalletLockDays=0)
    return TokenomicsResponse(
        totalSupply=row.total_supply or "0",
        burned=row.burned or "0%",
        devWalletLockDays=row.dev_wallet_lock_days or 0,
    )


def get_sentiment(db: Session, address: str) -> SentimentResponse | None:
    token_info = db.execute(select(TokenInfoModel).where(TokenInfoModel.contract_address == address)).scalars().first()
    if not token_info:
        gem = db.execute(select(GemModel).where(GemModel.contract_address == address)).scalars().first()
        if not gem:
            return None
        return SentimentResponse(score=gem.security_score or 0, label="Neutral", comments=[])
    s = db.execute(select(TokenSentimentModel).where(TokenSentimentModel.token_info_id == token_info.id)).scalars().first()
    if not s:
        return SentimentResponse(score=0, label="Neutral", comments=[])
    comments = s.comments or []
    from app.launchpad.schemas import SentimentComment

    sentiment_comments = [SentimentComment(author=x.get("author", ""), timestamp=x.get("timestamp", ""), text=x.get("text", "")) for x in comments if isinstance(x, dict)]
    return SentimentResponse(score=s.score, label=s.label or "Neutral", comments=sentiment_comments)


def add_sentiment_vote(db: Session, address: str, wallet_address: str, body: SentimentVoteBody) -> bool:
    """One vote per user per token; vote can be updated. Upsert to avoid race."""
    from uuid import uuid4
    from sqlalchemy import insert

    token_info = db.execute(select(TokenInfoModel).where(TokenInfoModel.contract_address == address)).scalars().first()
    if not token_info:
        return False
    # Upsert: one row per (contract_address, wallet_address)
    stmt = insert(SentimentVoteModel).values(
        id=str(uuid4()),
        contract_address=address,
        wallet_address=wallet_address,
        vote=body.vote,
    ).on_conflict_do_update(
        index_elements=["contract_address", "wallet_address"],
        set_={"vote": body.vote},
    )
    db.execute(stmt)
    db.flush()
    # Recalculate bullish/bearish counts from all votes and update aggregated sentiment
    votes = db.execute(
        select(SentimentVoteModel.vote).where(SentimentVoteModel.contract_address == address)
    ).scalars().all()
    bullish_count = sum(1 for v in votes if v == "bullish")
    bearish_count = sum(1 for v in votes if v == "bearish")
    total = bullish_count + bearish_count
    score = round((bullish_count / total * 100.0), 1) if total else 50.0
    label = "Bullish" if score >= 60 else ("Bearish" if score < 40 else "Neutral")
    stmt = insert(TokenSentimentModel).values(
        id=str(uuid4()),
        token_info_id=token_info.id,
        score=score,
        label=label,
        comments=[],
        bullish_count=bullish_count,
        bearish_count=bearish_count,
    ).on_conflict_do_update(
        index_elements=["token_info_id"],
        set_={
            "score": score,
            "label": label,
            "bullish_count": bullish_count,
            "bearish_count": bearish_count,
        },
    )
    db.execute(stmt)
    db.commit()
    return True


def get_price_history(
    db: Session,
    address: str,
    range_: str = "24h",
    points: int = 24,
    page: int = 1,
    limit: int | None = None,
) -> tuple[list[PriceHistoryPoint], int]:
    """Efficient query with optional pagination. Returns (items, total)."""
    base_q = select(PriceHistoryModel).where(PriceHistoryModel.contract_address == address)
    total = db.scalar(select(func.count()).select_from(PriceHistoryModel).where(PriceHistoryModel.contract_address == address)) or 0
    q = base_q.order_by(PriceHistoryModel.created_at.asc())
    if limit is not None and limit > 0:
        q = q.offset((page - 1) * limit).limit(limit)
    else:
        q = q.limit(points)
    rows = db.execute(q).scalars().all()
    items = [PriceHistoryPoint(time=r.time, price=r.price, volume=r.volume) for r in rows]
    return items, total


def get_orderbook(db: Session, address: str, sell_limit: int = 50, buy_limit: int = 50) -> OrderBookResponse:
    sell = (
        db.execute(
            select(OrderBookEntryModel)
            .where(OrderBookEntryModel.contract_address == address, OrderBookEntryModel.side == "sell")
            .order_by(OrderBookEntryModel.created_at.desc())
            .limit(sell_limit)
        )
        .scalars()
        .all()
        or []
    )
    buy = (
        db.execute(
            select(OrderBookEntryModel)
            .where(OrderBookEntryModel.contract_address == address, OrderBookEntryModel.side == "buy")
            .order_by(OrderBookEntryModel.created_at.desc())
            .limit(buy_limit)
        )
        .scalars()
        .all()
        or []
    )
    return OrderBookResponse(
        sellOffers=[OrderBookOffer(price=e.price, amount=e.amount, score=e.score, address=e.address) for e in sell],
        buyOffers=[OrderBookOffer(price=e.price, amount=e.amount, score=e.score, address=e.address) for e in buy],
    )


# ----- Audit -----
def get_audit(db: Session, address: str) -> AuditResponse | None:
    r = db.execute(select(AuditModel).where(AuditModel.contract_address == address)).scalar_one_or_none()
    if not r:
        return None
    from app.launchpad.schemas import (
        AuditCommentItem,
        CommunitySentimentSchema,
        LiquidityLockItem,
        LiquidityLocksSchema,
        SecurityCheckItem,
        VulnerabilitiesSchema,
    )

    checks = [SecurityCheckItem(name=x["name"], status=x["status"], description=x["description"], tooltip=x.get("tooltip")) for x in (r.security_checks or []) if isinstance(x, dict)]
    vuln = r.vulnerabilities or {}
    if isinstance(vuln, dict):
        vuln_s = VulnerabilitiesSchema(critical=vuln.get("critical", 0), high=vuln.get("high", 0), medium=vuln.get("medium", 0), low=vuln.get("low", 0))
    else:
        vuln_s = VulnerabilitiesSchema(critical=0, high=0, medium=0, low=0)
    locks_data = r.liquidity_locks or {}
    locks_list = locks_data.get("locks", []) if isinstance(locks_data, dict) else []
    locks_items = [LiquidityLockItem(lockerName=x.get("lockerName", ""), contractAddress=x.get("contractAddress", ""), amount=x.get("amount", ""), unlocksIn=x.get("unlocksIn", ""), unlockDate=x.get("unlockDate", ""), txHash=x.get("txHash", "")) for x in locks_list if isinstance(x, dict)]
    liq_locks = LiquidityLocksSchema(totalLocked=locks_data.get("totalLocked", "0%") if isinstance(locks_data, dict) else "0%", locks=locks_items)
    comm = r.community_sentiment or {}
    if isinstance(comm, dict):
        comm_comments = [AuditCommentItem(author=x.get("author", ""), reputation=x.get("reputation"), text=x.get("text", "")) for x in (comm.get("comments") or []) if isinstance(x, dict)]
        comm_s = CommunitySentimentSchema(bullish=comm.get("bullish", 0), bearish=comm.get("bearish", 0), upvotes=comm.get("upvotes", "0"), watchlists=comm.get("watchlists", "0"), comments=comm_comments)
    else:
        comm_s = CommunitySentimentSchema(bullish=0, bearish=0, upvotes="0", watchlists="0", comments=[])
    return AuditResponse(
        projectIcon=r.project_icon or "⭐",
        projectName=r.project_name or "",
        contractAddress=r.contract_address,
        fullAddress=r.full_address or r.contract_address,
        network=r.network or "",
        auditCompleted=r.audit_completed or "",
        isVerified=r.is_verified or False,
        verdict=r.verdict or "",
        riskLevel=r.risk_level or "",
        trustScore=r.trust_score or 0,
        trustSummary=r.trust_summary or "",
        securityChecks=checks,
        vulnerabilities=vuln_s,
        liquidityLocks=liq_locks,
        communitySentiment=comm_s,
        tokenSymbol=r.token_symbol or "",
    )


def add_audit_comment(db: Session, address: str, wallet_address: str, body: AuditCommentBody) -> dict | None:
    audit = db.execute(select(AuditModel).where(AuditModel.contract_address == address)).scalars().first()
    if not audit:
        return None
    c = AuditCommentModel(audit_id=audit.id, author=wallet_address, text=body.text)
    db.add(c)
    db.commit()
    db.refresh(c)
    return {"id": c.id, "author": c.author, "text": c.text, "createdAt": _iso(c.created_at)}


# Static markdown for public audit resources
AUDIT_METHODOLOGY_MD = """# Audit Methodology

## Overview

This document describes the methodology used for smart contract audits on the Launchpad.

## Scope

- **Smart contracts**: Token, presale, and liquidity contracts.
- **Checks**: Ownership renunciation, proxy detection, liquidity locks, vulnerability scan.

## Process

1. **Submission**: Project submits contract addresses and audit report link.
2. **Automated checks**: Security checks run against contract state.
3. **Manual review**: Critical findings are reviewed by the team.
4. **Verdict**: RUG-PROOF VERIFIED, WARN, or FAILED.

## Trust Score

The trust score (0–100) aggregates: security checks, liquidity lock status, and community sentiment.
"""

AUDIT_APPEAL_MD = """# Audit Appeal Process

## How to appeal

If you believe an audit result is incorrect or outdated:

1. **Contact**: Submit a request via the Launchpad support channel.
2. **Evidence**: Provide updated contract state, new audit report, or tx hashes.
3. **Review**: The team will re-run checks and update the verdict within 5 business days.
4. **Outcome**: You will receive an updated verdict and trust score, or a reason for denial.
"""


def get_audit_report_markdown(db: Session, address: str) -> str | None:
    """Returns audit report as markdown for the given contract address, or None if not found."""
    r = db.execute(select(AuditModel).where(AuditModel.contract_address == address)).scalars().first()
    if not r:
        return None
    lines = [
        f"# Audit Report: {r.project_name or 'Unknown'}",
        "",
        f"- **Contract**: `{r.contract_address}`",
        f"- **Network**: {r.network or 'N/A'}",
        f"- **Token**: {r.token_symbol or 'N/A'}",
        f"- **Completed**: {r.audit_completed or 'N/A'}",
        f"- **Verdict**: {r.verdict or 'N/A'}",
        f"- **Risk level**: {r.risk_level or 'N/A'}",
        f"- **Trust score**: {r.trust_score or 0}/100",
        "",
        "## Summary",
        "",
        (r.trust_summary or "No summary available."),
        "",
    ]
    if r.security_checks and isinstance(r.security_checks, list):
        lines.append("## Security checks")
        lines.append("")
        for c in r.security_checks:
            if isinstance(c, dict):
                name = c.get("name", "")
                status = c.get("status", "")
                desc = c.get("description", "")
                lines.append(f"- **{name}**: {status} — {desc}")
        lines.append("")
    return "\n".join(lines)


# ----- Watchlist -----
def get_watchlist(db: Session, wallet_address: str) -> list[str]:
    rows = db.execute(select(WatchlistModel).where(WatchlistModel.wallet_address == wallet_address)).scalars().all()
    return [r.contract_address for r in rows]


def add_to_watchlist(db: Session, wallet_address: str, contract_address: str) -> bool:
    """Add token to watchlist. Returns True if added, False if already present. Upsert-safe."""
    from uuid import uuid4
    from sqlalchemy import insert

    stmt = insert(WatchlistModel).values(
        id=str(uuid4()),
        wallet_address=wallet_address,
        contract_address=contract_address,
    ).on_conflict_do_nothing(index_elements=["wallet_address", "contract_address"])
    r = db.execute(stmt)
    db.commit()
    # rowcount: 1 if inserted, 0 if conflict (already present)
    return r.rowcount is not None and r.rowcount > 0


def remove_from_watchlist(db: Session, wallet_address: str, contract_address: str) -> bool:
    r = db.execute(select(WatchlistModel).where(WatchlistModel.wallet_address == wallet_address, WatchlistModel.contract_address == contract_address)).scalars().first()
    if r:
        db.delete(r)
        db.commit()
    return True


# ----- Submissions -----
def _submission_row_to_detail(r: SubmissionModel) -> dict:
    return {
        "id": r.id,
        "walletAddress": r.wallet_address,
        "projectName": r.project_name or "",
        "tokenSymbol": r.token_symbol or "",
        "totalSupply": r.total_supply or "",
        "launchSupply": r.launch_supply or "",
        "contactEmail": r.contact_email or "",
        "logoUrl": r.logo_url,
        "contractAddress": r.contract_address,
        "contractTokenAddress": r.contract_token_address,
        "chainId": r.chain_id,
        "network": r.network,
        "auditReport": r.audit_report or "",
        "twitter": r.twitter,
        "telegram": r.telegram,
        "status": r.status,
        "reviewedAt": _iso(r.reviewed_at) if r.reviewed_at else None,
        "reviewerNotes": r.reviewer_notes,
        "reviewerWallet": r.reviewer_wallet,
        "createdAt": _iso(r.created_at),
    }


def create_submission(db: Session, wallet_address: str, body: SubmissionPostBody) -> dict:
    s = SubmissionModel(
        wallet_address=wallet_address,
        contract_address=body.contractAddress,
        contract_token_address=body.contractTokenAddress,
        network=body.network,
        chain_id=body.chainId,
        project_name=body.projectName,
        token_symbol=body.tokenSymbol,
        total_supply=body.totalSupply,
        launch_supply=body.launchSupply,
        logo_url=body.logoUrl,
        contact_email=body.contactEmail,
        audit_report=body.auditReport or "",
        twitter=body.twitter,
        telegram=body.telegram,
        status="pending",
    )
    db.add(s)
    db.commit()
    db.refresh(s)
    return {"id": s.id, "status": s.status, "createdAt": _iso(s.created_at)}


def get_submission(db: Session, submission_id: str, wallet_address: str) -> dict | None:
    r = db.execute(select(SubmissionModel).where(SubmissionModel.id == submission_id, SubmissionModel.wallet_address == wallet_address)).scalar_one_or_none()
    if not r:
        return None
    return _submission_row_to_detail(r)


def get_submission_by_id(db: Session, submission_id: str) -> dict | None:
    r = db.execute(select(SubmissionModel).where(SubmissionModel.id == submission_id)).scalar_one_or_none()
    if not r:
        return None
    return _submission_row_to_detail(r)


def list_submissions_mine(db: Session, wallet_address: str, page: int = 1, limit: int = 20) -> tuple[list[dict], int]:
    count_q = select(func.count()).select_from(SubmissionModel).where(SubmissionModel.wallet_address == wallet_address)
    total = db.scalar(count_q) or 0
    q = select(SubmissionModel).where(SubmissionModel.wallet_address == wallet_address).order_by(desc(SubmissionModel.created_at)).offset((page - 1) * limit).limit(limit)
    rows = db.execute(q).scalars().all()
    data = [
        {
            "id": r.id,
            "projectName": r.project_name or "",
            "contractAddress": r.contract_address,
            "network": r.network,
            "status": r.status,
            "createdAt": _iso(r.created_at),
        }
        for r in rows
    ]
    return data, total


def list_submissions_admin(
    db: Session, status: str | None = None, page: int = 1, limit: int = 20
) -> tuple[list[dict], int]:
    count_q = select(func.count()).select_from(SubmissionModel)
    if status:
        count_q = count_q.where(SubmissionModel.status == status)
    total = db.scalar(count_q) or 0
    q = select(SubmissionModel)
    if status:
        q = q.where(SubmissionModel.status == status)
    q = q.order_by(desc(SubmissionModel.created_at)).offset((page - 1) * limit).limit(limit)
    rows = db.execute(q).scalars().all()
    data = [_submission_row_to_detail(r) for r in rows]
    return data, total


def review_submission(
    db: Session,
    submission_id: str,
    decision: str,
    reviewer_notes: str | None,
    reviewer_wallet: str,
) -> dict | None:
    r = db.execute(select(SubmissionModel).where(SubmissionModel.id == submission_id)).scalar_one_or_none()
    if not r:
        return None
    if r.status != "pending":
        raise ValueError("submission_already_reviewed")
    r.status = "approved" if decision == "approve" else "rejected"
    r.reviewed_at = datetime.now(timezone.utc)
    r.reviewer_notes = reviewer_notes
    r.reviewer_wallet = reviewer_wallet
    db.commit()
    db.refresh(r)
    return _submission_row_to_detail(r)



