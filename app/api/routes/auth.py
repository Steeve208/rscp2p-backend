"""
Auth Face ID: challenge/verify + JWT.
POST /api/auth/challenge, /api/auth/verify, /api/auth/refresh, GET /api/auth/me, POST /api/auth/logout.
"""

import jwt
from fastapi import APIRouter, Depends, HTTPException
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer
from sqlalchemy.orm import Session

from app.config import settings
from app.db import get_db
from app.schemas.auth import (
    ChallengeRequest,
    ChallengeResponse,
    RefreshRequest,
    RefreshResponse,
    UserResponse,
    VerifyRequest,
    VerifyResponse,
)
from app.services import auth as auth_service

security = HTTPBearer(auto_error=False)


def _get_token(credentials: HTTPAuthorizationCredentials | None) -> str | None:
    if credentials is None:
        return None
    return credentials.credentials


def get_current_user(
    credentials: HTTPAuthorizationCredentials | None = Depends(security),
    db: Session = Depends(get_db),
) -> UserResponse:
    token = _get_token(credentials)
    if not token:
        raise HTTPException(status_code=401, detail="Missing or invalid token")
    try:
        payload = jwt.decode(
            token,
            settings.jwt_secret,
            algorithms=[settings.jwt_algorithm],
        )
        if payload.get("type") != "access":
            raise HTTPException(status_code=401, detail="Invalid token type")
        wallet = payload.get("sub")
        if not wallet:
            raise HTTPException(status_code=401, detail="Invalid token")
        user = auth_service.get_user_by_wallet(db, wallet)
        if not user:
            raise HTTPException(status_code=401, detail="User not found")
        return user
    except jwt.PyJWTError:
        raise HTTPException(status_code=401, detail="Invalid or expired token")


def get_current_user_optional(
    credentials: HTTPAuthorizationCredentials | None = Depends(security),
    db: Session = Depends(get_db),
) -> UserResponse | None:
    """Like get_current_user but returns None instead of 401 when unauthenticated."""
    token = _get_token(credentials)
    if not token:
        return None
    try:
        payload = jwt.decode(
            token,
            settings.jwt_secret,
            algorithms=[settings.jwt_algorithm],
        )
        if payload.get("type") != "access":
            return None
        wallet = payload.get("sub")
        if not wallet:
            return None
        return auth_service.get_user_by_wallet(db, wallet)
    except jwt.PyJWTError:
        return None


def require_admin(user: UserResponse = Depends(get_current_user)) -> UserResponse:
    """Solo wallets listadas en ADMIN_WALLET_ADDRESSES (config)."""
    allow = settings.admin_wallet_set
    if not allow:
        raise HTTPException(status_code=503, detail="Admin list not configured")
    if user.walletAddress.lower() not in allow:
        raise HTTPException(status_code=403, detail="Admin access required")
    return user


router = APIRouter(prefix="/auth", tags=["auth"])


@router.post("/challenge", response_model=ChallengeResponse)
def auth_challenge(body: ChallengeRequest, db: Session = Depends(get_db)):
    return auth_service.challenge(db, body.walletAddress)


@router.post("/verify", response_model=VerifyResponse)
def auth_verify(body: VerifyRequest, db: Session = Depends(get_db)):
    result = auth_service.verify(db, body.walletAddress, body.nonce, body.signature)
    if result is None:
        raise HTTPException(status_code=401, detail="Invalid signature or nonce")
    return result


@router.post("/refresh", response_model=RefreshResponse)
def auth_refresh(body: RefreshRequest, db: Session = Depends(get_db)):
    if not body.refreshToken:
        raise HTTPException(status_code=400, detail="refreshToken required")
    access_token = auth_service.refresh_token(db, body.refreshToken)
    if not access_token:
        raise HTTPException(status_code=401, detail="Invalid or expired refresh token")
    return RefreshResponse(accessToken=access_token)


@router.get("/me", response_model=UserResponse)
def auth_me(user: UserResponse = Depends(get_current_user)):
    return user


@router.post("/logout")
def auth_logout(user: UserResponse = Depends(get_current_user)):
    return {"ok": True}
