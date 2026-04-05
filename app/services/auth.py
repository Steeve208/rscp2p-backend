"""
Auth Face ID: challenge/verify con firma de wallet + JWT.
Usuarios y nonces persistidos en BD para soporte multi-instancia.
"""

import time
from datetime import datetime, timedelta, timezone
from uuid import uuid4

import jwt
from eth_account import Account
from eth_account.messages import encode_defunct
from sqlalchemy import delete, select
from sqlalchemy.orm import Session

from app.config import settings
from app.models.auth_nonces import AuthNonceModel
from app.models.marketplace import UserModel
from app.schemas.auth import (
    ChallengeResponse,
    UserResponse,
    VerifyResponse,
)

Account.enable_unaudited_hdwallet_features()

MESSAGE_TEMPLATE = "Login to RSC P2P Terminal\nNonce: {nonce}"
NONCE_TTL_MINUTES = 5


def _now_iso() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def _user_to_response(u: UserModel) -> UserResponse:
    def _iso(dt: datetime | None) -> str | None:
        if dt is None:
            return None
        if dt.tzinfo is None:
            dt = dt.replace(tzinfo=timezone.utc)
        return dt.astimezone(timezone.utc).isoformat().replace("+00:00", "Z")

    return UserResponse(
        id=u.id,
        walletAddress=u.wallet_address,
        reputationScore=float(u.reputation_score),
        isActive=u.is_active,
        loginCount=u.login_count or 0,
        lastLoginAt=_iso(u.last_login_at),
        createdAt=_iso(u.created_at) or _now_iso(),
    )


def challenge(db: Session, wallet_address: str) -> ChallengeResponse:
    wallet_address = wallet_address.strip().lower()
    nonce = str(uuid4())
    message = MESSAGE_TEMPLATE.format(nonce=nonce)
    expires_at = datetime.now(timezone.utc) + timedelta(minutes=NONCE_TTL_MINUTES)
    # Replace any existing nonce for this wallet
    db.execute(delete(AuthNonceModel).where(AuthNonceModel.wallet_address == wallet_address))
    db.add(AuthNonceModel(wallet_address=wallet_address, nonce=nonce, expires_at=expires_at))
    db.commit()
    return ChallengeResponse(nonce=nonce, message=message)


def verify(db: Session, wallet_address: str, nonce: str, signature: str) -> VerifyResponse | None:
    wallet_address = wallet_address.strip().lower()
    row = db.execute(
        select(AuthNonceModel).where(
            AuthNonceModel.wallet_address == wallet_address,
            AuthNonceModel.expires_at > datetime.now(timezone.utc),
        ).limit(1)
    ).scalar_one_or_none()
    if not row or row.nonce != nonce:
        return None
    db.execute(delete(AuthNonceModel).where(AuthNonceModel.wallet_address == wallet_address))
    db.commit()
    message = MESSAGE_TEMPLATE.format(nonce=nonce)
    try:
        message_encoded = encode_defunct(text=message)
        recovered = Account.recover_message(message_encoded, signature=signature)
        if recovered.lower() != wallet_address:
            return None
    except Exception:
        return None

    now = datetime.now(timezone.utc)
    u = db.scalar(select(UserModel).where(UserModel.wallet_address == wallet_address).limit(1))
    if u is None:
        u = UserModel(
            id=str(uuid4()),
            wallet_address=wallet_address,
            reputation_score=0.0,
            is_active=True,
            login_count=1,
            last_login_at=now,
        )
        db.add(u)
    else:
        u.login_count = (u.login_count or 0) + 1
        u.last_login_at = now
    db.commit()
    db.refresh(u)

    user_response = _user_to_response(u)

    access_payload = {
        "sub": wallet_address,
        "type": "access",
        "exp": int(time.time()) + settings.jwt_access_expire_minutes * 60,
        "iat": int(time.time()),
    }
    refresh_payload = {
        "sub": wallet_address,
        "type": "refresh",
        "exp": int(time.time()) + settings.jwt_refresh_expire_days * 86400,
        "iat": int(time.time()),
    }
    access_token = jwt.encode(
        access_payload, settings.jwt_secret, algorithm=settings.jwt_algorithm
    )
    refresh_token = jwt.encode(
        refresh_payload, settings.jwt_secret, algorithm=settings.jwt_algorithm
    )

    return VerifyResponse(
        accessToken=access_token,
        refreshToken=refresh_token,
        user=user_response,
    )


def refresh_token(db: Session, refresh_token_str: str) -> str | None:
    try:
        payload = jwt.decode(
            refresh_token_str,
            settings.jwt_secret,
            algorithms=[settings.jwt_algorithm],
        )
        if payload.get("type") != "refresh":
            return None
        wallet_address = payload.get("sub")
        if not wallet_address:
            return None
        u = db.scalar(select(UserModel).where(UserModel.wallet_address == wallet_address).limit(1))
        if u is None:
            return None
        access_payload = {
            "sub": wallet_address,
            "type": "access",
            "exp": int(time.time()) + settings.jwt_access_expire_minutes * 60,
            "iat": int(time.time()),
        }
        return jwt.encode(
            access_payload, settings.jwt_secret, algorithm=settings.jwt_algorithm
        )
    except Exception:
        return None


def get_user_by_wallet(db: Session, wallet_address: str) -> UserResponse | None:
    wallet_address = wallet_address.strip().lower()
    u = db.scalar(select(UserModel).where(UserModel.wallet_address == wallet_address).limit(1))
    if u is None:
        return None
    return _user_to_response(u)
