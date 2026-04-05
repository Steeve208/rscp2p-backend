"""
Idempotency layer for order mutation endpoints.
Guarantees at-most-once execution: duplicate requests (same Idempotency-Key) return stored result.
Single DB transaction: claim key (flush) + fn(commit=False) + persist response + commit.
Orphan recovery: keys older than IDEMPOTENCY_ORPHAN_MINUTES with no result are marked expired so client can retry with new key.
"""

import hashlib
import json
from datetime import datetime, timezone, timedelta
from typing import Callable, TypeVar

from fastapi import HTTPException
from sqlalchemy.exc import IntegrityError
from sqlalchemy.orm import Session

from app.models.idempotency import IdempotencyKeyModel

IDEMPOTENCY_ORPHAN_MINUTES = 5

T = TypeVar("T")

# Endpoint names for idempotency_keys.endpoint / event_type
ENDPOINT_ACCEPT = "orders/accept"
ENDPOINT_MARK_LOCKED = "orders/mark-locked"
ENDPOINT_COMPLETE = "orders/complete"
ENDPOINT_CANCEL = "orders/cancel"
ENDPOINT_DISPUTE = "orders/dispute"


def _hash_snapshot(snapshot: str) -> str:
    return hashlib.sha256(snapshot.encode("utf-8")).hexdigest()[:64]


def run_idempotent(
    db: Session,
    idempotency_key: str,
    endpoint: str,
    order_id: str,
    fn: Callable[[], T],
    serialize: Callable[[T], dict],
) -> tuple[int, dict]:
    """
    Execute fn at most once per idempotency_key. Duplicate requests return cached (status, body).

    One transaction for the happy path: INSERT idempotency row (flush) → fn() without internal
    commit → UPDATE row with JSON snapshot → commit. fn() must use commit=False on order mutators.

    Returns (http_status_code, body_dict). Caller should return JSONResponse(content=body_dict, status_code=status).
    """
    row: IdempotencyKeyModel | None = None
    try:
        row = IdempotencyKeyModel(
            idempotency_key=idempotency_key,
            order_id=order_id,
            event_type=endpoint,
            endpoint=endpoint,
            response_status=None,
            result_snapshot=None,
            response_hash=None,
        )
        db.add(row)
        db.flush()
    except IntegrityError:
        db.rollback()
        existing = (
            db.query(IdempotencyKeyModel)
            .filter(IdempotencyKeyModel.idempotency_key == idempotency_key)
            .first()
        )
        if existing is None:
            raise HTTPException(
                status_code=500,
                detail="Idempotency key conflict (race)",
            )
        if existing.response_status is not None:
            body = {}
            if existing.result_snapshot:
                try:
                    body = json.loads(existing.result_snapshot)
                except (json.JSONDecodeError, TypeError):
                    body = {"detail": existing.result_snapshot}
            return (existing.response_status, body)
        cutoff = datetime.now(timezone.utc) - timedelta(minutes=IDEMPOTENCY_ORPHAN_MINUTES)
        if existing.created_at and existing.created_at < cutoff:
            existing.response_status = 409
            existing.result_snapshot = json.dumps({
                "detail": "Idempotent request expired; retry with a new Idempotency-Key",
            })
            db.commit()
            return (409, {"detail": "Idempotent request expired; retry with a new Idempotency-Key"})
        raise HTTPException(
            status_code=409,
            detail="Idempotent request in progress; retry after a short delay",
        )

    assert row is not None

    try:
        result = fn()
        status = 200
        body = serialize(result)
    except HTTPException as e:
        status = e.status_code
        body = {"detail": e.detail if isinstance(e.detail, str) else str(e.detail)}
        snapshot = json.dumps(body)
        row.result_snapshot = snapshot
        row.response_status = status
        row.response_hash = _hash_snapshot(snapshot)
        db.commit()
        return (status, body)
    except Exception:
        db.rollback()
        raise

    snapshot = json.dumps(body)
    row.result_snapshot = snapshot
    row.response_status = status
    row.response_hash = _hash_snapshot(snapshot)
    db.commit()

    return (status, body)
