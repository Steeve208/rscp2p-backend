"""
Schemas para auth (Face ID flow: challenge/verify + JWT).
Alineados con frontend: AuthChallengeResponse, AuthVerifyResponse, User.
"""

from pydantic import BaseModel, ConfigDict, Field


class ChallengeRequest(BaseModel):
    model_config = ConfigDict(strict=True, str_strip_whitespace=True)

    walletAddress: str = Field(..., min_length=10, max_length=66, description="Dirección de wallet del usuario")


class ChallengeResponse(BaseModel):
    nonce: str
    message: str


class VerifyRequest(BaseModel):
    model_config = ConfigDict(strict=True, str_strip_whitespace=True)

    walletAddress: str = Field(..., min_length=10, max_length=66)
    nonce: str = Field(..., min_length=1, max_length=64)
    signature: str = Field(..., min_length=1, max_length=200)


class UserResponse(BaseModel):
    id: str
    walletAddress: str
    reputationScore: float = 0
    isActive: bool = True
    loginCount: int = 0
    lastLoginAt: str | None = None
    createdAt: str


class VerifyResponse(BaseModel):
    accessToken: str
    refreshToken: str
    user: UserResponse


class RefreshRequest(BaseModel):
    refreshToken: str


class RefreshResponse(BaseModel):
    accessToken: str
