"""
Rutas Launchpad: /api/launchpad/*.
Todos los endpoints definidos en docs/LAUNCHPAD_ESTUDIO_BACKEND.md.
"""

from fastapi import APIRouter, Depends, HTTPException, Query
from fastapi.responses import PlainTextResponse
from sqlalchemy.orm import Session

from app.api.routes.auth import get_current_user, get_current_user_optional, require_admin
from app.schemas.auth import UserResponse
from app.db import get_db
from app.launchpad import service as svc
from app.websocket.socketio import notify_presale_contribution
from app.launchpad.schemas import (
    AuditCommentBody,
    ContributionResponse,
    FeaturedGemResponse,
    GemResponse,
    GlobalStatsResponse,
    OrderBookResponse,
    PresaleContributionItem,
    PresaleResponse,
    PostPresaleContributionBody,
    PriceHistoryPoint,
    SentimentResponse,
    SentimentVoteBody,
    SubmissionPostBody,
    SubmissionReviewBody,
    TokenDetailResponse,
    TokenomicsResponse,
    WatchlistPostBody,
)

router = APIRouter(prefix="/launchpad", tags=["launchpad"])


def _paginated(data: list, total: int, page: int, limit: int):
    total_pages = max(1, (total + limit - 1) // limit)
    return {"data": data, "total": total, "page": page, "limit": limit, "totalPages": total_pages}


# ----- 3.2 Gems -----
@router.get("/gems", response_model=None)
def list_gems(
    db: Session = Depends(get_db),
    page: int = Query(1, ge=1),
    limit: int = Query(20, ge=1, le=100),
    category: str | None = None,
    search: str | None = None,
    verified: bool | None = None,
    rugChecked: bool | None = None,
    minScore: float | None = None,
    maxScore: float | None = None,
):
    items, total = svc.list_gems(db, page=page, limit=limit, category=category, search=search, verified=verified, rug_checked=rugChecked, min_score=minScore, max_score=maxScore)
    return _paginated(items, total, page, limit)


@router.get("/gems/featured", response_model=None)
def get_featured(db: Session = Depends(get_db)):
    gem = svc.get_featured_gem(db)
    if gem is None:
        raise HTTPException(status_code=404, detail="No featured gem")
    return {"data": gem}


@router.get("/gems/stats", response_model=None)
def get_stats(db: Session = Depends(get_db)):
    stats = svc.get_global_stats(db)
    return {"data": stats}


@router.get("/gems/{address}", response_model=None)
def get_gem_by_address(address: str, db: Session = Depends(get_db)):
    gem = svc.get_gem_by_address(db, address)
    if gem is None:
        raise HTTPException(status_code=404, detail="Gem not found")
    return {"data": gem}


# ----- 3.3 Presales -----
@router.get("/presales", response_model=None)
def list_presales(
    db: Session = Depends(get_db),
    status: str | None = None,
    page: int = Query(1, ge=1),
    limit: int = Query(20, ge=1, le=100),
    search: str | None = None,
):
    items, total = svc.list_presales(db, status=status, page=page, limit=limit, search=search)
    return _paginated(items, total, page, limit)


@router.get("/presales/{presale_id}", response_model=None)
def get_presale(presale_id: str, db: Session = Depends(get_db)):
    presale = svc.get_presale_by_id(db, presale_id)
    if presale is None:
        raise HTTPException(status_code=404, detail="Presale not found")
    return {"data": presale}


@router.get("/presales/{presale_id}/contributions", response_model=None)
def get_presale_contributions(
    presale_id: str,
    db: Session = Depends(get_db),
    limit: int = Query(20, ge=1, le=100),
    user: UserResponse | None = Depends(get_current_user_optional),
):
    items = svc.get_presale_contributions(db, presale_id, limit=limit, mask_wallets=(user is None))
    return {"data": items}


@router.post("/presales/{presale_id}/contributions", response_model=None)
def post_presale_contribution(
    presale_id: str,
    body: PostPresaleContributionBody,
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
):
    if not (user.walletAddress or "").strip():
        raise HTTPException(
            status_code=400,
            detail="Authenticated user must have a wallet address to contribute",
        )
    if body.walletAddress and body.walletAddress.strip().lower() != (user.walletAddress or "").strip().lower():
        raise HTTPException(
            status_code=400,
            detail="walletAddress in body must match authenticated user",
        )
    contrib = svc.create_presale_contribution(db, presale_id, body, wallet_address=user.walletAddress or "")
    if contrib is None:
        raise HTTPException(status_code=404, detail="Presale not found")
    notify_presale_contribution(
        presale_id,
        {
            "id": contrib.id,
            "walletAddress": contrib.walletAddress,
            "amount": contrib.amount,
            "timestamp": contrib.createdAt or "",
        },
    )
    return {"data": contrib}


# ----- 3.4 Tokens -----
@router.get("/tokens/{address}", response_model=None)
def get_token_detail(address: str, db: Session = Depends(get_db)):
    token = svc.get_token_detail(db, address)
    if token is None:
        raise HTTPException(status_code=404, detail="Token not found")
    return {"data": token}


@router.get("/tokens/{address}/price-history", response_model=None)
def get_price_history(
    address: str,
    db: Session = Depends(get_db),
    range: str = Query("24h"),
    points: int = Query(24, ge=1, le=200),
    page: int = Query(1, ge=1),
    limit: int | None = Query(None, ge=1, le=200),
):
    items, total = svc.get_price_history(db, address, range_=range, points=points, page=page, limit=limit)
    if limit is not None:
        return {"data": items, "total": total, "page": page, "limit": limit}
    return {"data": items}


@router.get("/tokens/{address}/orderbook", response_model=None)
def get_orderbook(
    address: str,
    db: Session = Depends(get_db),
    limit: int = Query(50, ge=1, le=200),
):
    ob = svc.get_orderbook(db, address, sell_limit=limit, buy_limit=limit)
    return {"data": ob}


@router.get("/tokens/{address}/tokenomics", response_model=None)
def get_tokenomics(address: str, db: Session = Depends(get_db)):
    tok = svc.get_tokenomics(db, address)
    if tok is None:
        raise HTTPException(status_code=404, detail="Token not found")
    return {"data": tok}


@router.get("/tokens/{address}/sentiment", response_model=None)
def get_sentiment(address: str, db: Session = Depends(get_db)):
    sent = svc.get_sentiment(db, address)
    if sent is None:
        raise HTTPException(status_code=404, detail="Token not found")
    return {"data": sent}


@router.post("/tokens/{address}/sentiment/vote")
def post_sentiment_vote(
    address: str,
    body: SentimentVoteBody,
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
):
    ok = svc.add_sentiment_vote(db, address, user.walletAddress, body)
    if not ok:
        raise HTTPException(status_code=404, detail="Token not found")
    return {}


# ----- 3.5 Contributions (portfolio) -----
@router.get("/contributions/me", response_model=None)
def list_contributions_me(
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
    status: str | None = None,
    search: str | None = None,
    page: int = Query(1, ge=1),
    limit: int = Query(20, ge=1, le=100),
):
    items, total = svc.list_contributions_me(db, user.walletAddress, status=status, search=search, page=page, limit=limit)
    return _paginated(items, total, page, limit)


@router.get("/contributions/{contribution_id}", response_model=None)
def get_contribution(contribution_id: str, db: Session = Depends(get_db), user: UserResponse = Depends(get_current_user)):
    contrib = svc.get_contribution_by_id(db, contribution_id, user.walletAddress)
    if contrib is None:
        raise HTTPException(status_code=404, detail="Contribution not found")
    return {"data": contrib}


@router.get("/contributions/by-tx/{tx_hash}", response_model=None)
def get_contribution_by_tx(tx_hash: str, db: Session = Depends(get_db), user: UserResponse = Depends(get_current_user)):
    contrib = svc.get_contribution_by_tx(db, tx_hash, user.walletAddress)
    if contrib is None:
        raise HTTPException(status_code=404, detail="Contribution not found")
    return {"data": contrib}


# ----- 3.6 Audit -----
# Static audit routes first so /audit/methodology and /audit/appeal are not captured by {address}
@router.get("/audit/methodology", response_class=PlainTextResponse)
def get_audit_methodology():
    """Returns static audit methodology document (markdown)."""
    return PlainTextResponse(content=svc.AUDIT_METHODOLOGY_MD, media_type="text/markdown")


@router.get("/audit/appeal", response_class=PlainTextResponse)
def get_audit_appeal():
    """Returns static audit appeal process document (markdown)."""
    return PlainTextResponse(content=svc.AUDIT_APPEAL_MD, media_type="text/markdown")


@router.get("/audit/{address}", response_model=None)
def get_audit(address: str, db: Session = Depends(get_db)):
    audit = svc.get_audit(db, address)
    if audit is None:
        raise HTTPException(status_code=404, detail="Audit not found")
    return {"data": audit}


@router.get("/audit/{address}/report", response_class=PlainTextResponse)
def get_audit_report(address: str, db: Session = Depends(get_db)):
    """Returns audit report as markdown for the given contract address."""
    content = svc.get_audit_report_markdown(db, address)
    if content is None:
        raise HTTPException(status_code=404, detail="Audit not found")
    return PlainTextResponse(content=content, media_type="text/markdown")


@router.post("/audit/{address}/comment", response_model=None)
def post_audit_comment(
    address: str,
    body: AuditCommentBody,
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
):
    created = svc.add_audit_comment(db, address, user.walletAddress, body)
    if created is None:
        raise HTTPException(status_code=404, detail="Audit not found")
    return {"data": created}


# ----- 3.7 Watchlist -----
@router.get("/watchlist", response_model=None)
def get_watchlist(db: Session = Depends(get_db), user: UserResponse = Depends(get_current_user)):
    addresses = svc.get_watchlist(db, user.walletAddress)
    return {"data": addresses}


@router.post("/watchlist")
def post_watchlist(
    body: WatchlistPostBody,
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
):
    added = svc.add_to_watchlist(db, user.walletAddress, body.contractAddress)
    return {"data": {"added": added}}


@router.delete("/watchlist/{address}")
def delete_watchlist(address: str, db: Session = Depends(get_db), user: UserResponse = Depends(get_current_user)):
    svc.remove_from_watchlist(db, user.walletAddress, address)
    return {}


# ----- 3.8 Submissions -----
@router.post("/submissions", response_model=None)
def post_submission(
    body: SubmissionPostBody,
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
):
    created = svc.create_submission(db, user.walletAddress, body)
    return {"data": created}


@router.get("/submissions/{submission_id}", response_model=None)
def get_submission(submission_id: str, db: Session = Depends(get_db), user: UserResponse = Depends(get_current_user)):
    sub = svc.get_submission(db, submission_id, user.walletAddress)
    if sub is None:
        raise HTTPException(status_code=404, detail="Submission not found")
    return {"data": sub}


@router.get("/submissions/mine", response_model=None)
def list_submissions_mine(
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
    page: int = Query(1, ge=1),
    limit: int = Query(20, ge=1, le=100),
):
    items, total = svc.list_submissions_mine(db, user.walletAddress, page=page, limit=limit)
    return _paginated(items, total, page, limit)


# ----- Admin: revisiones de envíos al Launchpad -----
@router.get("/admin/submissions", response_model=None)
def admin_list_submissions(
    db: Session = Depends(get_db),
    admin: UserResponse = Depends(require_admin),
    status: str | None = Query(None, description="pending | approved | rejected"),
    page: int = Query(1, ge=1),
    limit: int = Query(20, ge=1, le=100),
):
    _ = admin
    items, total = svc.list_submissions_admin(db, status=status, page=page, limit=limit)
    return _paginated(items, total, page, limit)


@router.get("/admin/submissions/{submission_id}", response_model=None)
def admin_get_submission(
    submission_id: str,
    db: Session = Depends(get_db),
    admin: UserResponse = Depends(require_admin),
):
    _ = admin
    sub = svc.get_submission_by_id(db, submission_id)
    if sub is None:
        raise HTTPException(status_code=404, detail="Submission not found")
    return {"data": sub}


@router.post("/admin/submissions/{submission_id}/review", response_model=None)
def admin_review_submission(
    submission_id: str,
    body: SubmissionReviewBody,
    db: Session = Depends(get_db),
    admin: UserResponse = Depends(require_admin),
):
    try:
        updated = svc.review_submission(
            db,
            submission_id,
            body.decision,
            body.reviewerNotes,
            admin.walletAddress,
        )
    except ValueError as e:
        if str(e) == "submission_already_reviewed":
            raise HTTPException(status_code=409, detail="Submission already reviewed") from e
        raise
    if updated is None:
        raise HTTPException(status_code=404, detail="Submission not found")
    return {"data": updated}
