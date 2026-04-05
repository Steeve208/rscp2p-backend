"""Common API dependencies."""

import re
from uuid import UUID

from fastapi import Header, HTTPException


def require_idempotency_key(
    idempotency_key: str = Header(..., alias="Idempotency-Key", description="UUID for idempotent request"),
) -> str:
    """Require Idempotency-Key header (UUID). Used by order mutation endpoints."""
    key = (idempotency_key or "").strip()
    if not key:
        raise HTTPException(
            status_code=400,
            detail="Missing required header: Idempotency-Key",
        )
    # Allow UUID format (with or without hyphens)
    if not re.match(r"^[0-9a-fA-F-]{32,36}$", key):
        raise HTTPException(
            status_code=400,
            detail="Idempotency-Key must be a valid UUID",
        )
    return key
