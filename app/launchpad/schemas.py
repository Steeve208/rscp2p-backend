"""
Schemas Pydantic para Launchpad.
Alineados con docs/LAUNCHPAD_ESTUDIO_BACKEND.md (tipos 5.1 a 5.7 y endpoints).
"""

import re
from decimal import Decimal
from typing import Literal

from pydantic import BaseModel, Field, field_validator


# ----- 5.1 Gem -----
class GemResponse(BaseModel):
    projectIcon: str
    projectName: str
    description: str
    securityScore: float
    priceChange: float
    liquidityDepth: str
    upvotes: str
    launchDate: str
    sparklineData: list[float]
    contractAddress: str
    category: str | None = None
    isVerified: bool | None = None
    rugChecked: bool | None = None
    price: float | None = None
    volume24h: float | None = None


# ----- 5.2 FeaturedGem -----
class FeaturedParticipant(BaseModel):
    address: str
    amount: float | None = None
    timestamp: str | None = None


class FeaturedGemResponse(BaseModel):
    projectName: str
    subtitle: str
    description: str
    endTime: str
    contractAddress: str
    projectIcon: str | None = None
    category: str | None = None
    raised: float | None = None
    target: float | None = None
    participants: list[FeaturedParticipant] | None = None
    watchingCount: int | None = None
    trendingRank: int | None = None


# ----- 5.3 GlobalStats -----
class GlobalStatsResponse(BaseModel):
    totalGems: int
    totalLiquidity: str
    avgSecurityScore: float
    activePresales: int
    totalVolume24h: str | None = None


# ----- 5.4 Presale -----
class VestingTerms(BaseModel):
    tgeUnlock: str
    cliffPeriod: str
    linearVesting: str
    totalMonths: int | None = None


class PresaleResponse(BaseModel):
    id: str
    projectName: str
    projectDescription: str
    projectIcon: str
    isVerified: bool
    contractAddress: str
    tokenSymbol: str
    exchangeRate: float
    minBuy: str
    maxBuy: str
    endDate: str
    softCap: str
    hardCap: str
    minContrib: str
    maxContrib: str
    vestingTerms: VestingTerms
    auditUrl: str | None = None
    contractUrl: str | None = None
    status: str = "active"


# ----- Presale contributions (feed) -----
class PresaleContributionItem(BaseModel):
    id: str
    walletAddress: str
    amount: str
    timestamp: str


class PostPresaleContributionBody(BaseModel):
    """walletAddress is ignored; server uses JWT identity. Kept optional for backward compatibility."""
    walletAddress: str | None = None
    amount: str
    txHash: str

    @field_validator("amount")
    @classmethod
    def amount_positive(cls, v: str) -> str:
        if not v or not v.strip():
            raise ValueError("amount is required")
        try:
            d = Decimal(v.strip())
            if d <= 0:
                raise ValueError("amount must be positive")
        except Exception as e:
            if isinstance(e, ValueError) and "positive" in str(e):
                raise
            raise ValueError("amount must be a valid positive number")
        return v.strip()

    @field_validator("txHash")
    @classmethod
    def tx_hash_format(cls, v: str) -> str:
        if not v or not v.strip():
            raise ValueError("txHash is required")
        v = v.strip()
        if len(v) < 10 or len(v) > 66:
            raise ValueError("txHash must be 10-66 characters")
        if not re.match(r"^0x[0-9a-fA-F]+$", v) and not re.match(r"^[0-9a-fA-F]+$", v):
            raise ValueError("txHash must be hex (optional 0x prefix)")
        return v


# ----- 5.5 Contribution -----
class ContributionResponse(BaseModel):
    id: str
    walletAddress: str
    projectName: str
    projectIcon: str
    tokenSymbol: str
    presaleId: str
    contribution: str
    buyPrice: str
    currentValue: str
    growth: str
    isLoss: bool
    vestingProgress: float
    nextUnlock: str
    claimableAmount: str | None
    status: Literal["active", "fully-vested"]
    txHash: str | None = None
    createdAt: str | None = None


# ----- 5.6 TokenDetail -----
class TokenomicsSchema(BaseModel):
    totalSupply: str
    burned: str
    devWalletLockDays: int


class SentimentComment(BaseModel):
    author: str
    timestamp: str
    text: str


class DaoSentimentSchema(BaseModel):
    score: float
    label: str
    comments: list[SentimentComment]


class TokenDetailResponse(BaseModel):
    projectIcon: str
    projectName: str
    symbol: str
    price: float
    priceChange24h: float
    isVerified: bool
    contractAddress: str
    exchangeRate: float
    sparklineData: list[float]
    tokenomics: TokenomicsSchema
    daoSentiment: DaoSentimentSchema


class PriceHistoryPoint(BaseModel):
    time: str
    price: float
    volume: float | None = None


class OrderBookOffer(BaseModel):
    price: str
    amount: str
    score: float
    address: str


class OrderBookResponse(BaseModel):
    sellOffers: list[OrderBookOffer]
    buyOffers: list[OrderBookOffer]


class TokenomicsResponse(BaseModel):
    totalSupply: str
    burned: str
    devWalletLockDays: int


class SentimentResponse(BaseModel):
    score: float
    label: str
    comments: list[SentimentComment]


class SentimentVoteBody(BaseModel):
    vote: Literal["bullish", "bearish"]


# ----- 5.7 Audit -----
class SecurityCheckItem(BaseModel):
    name: str
    status: Literal["PASSED", "FAILED", "STABLE", "WARN"]
    description: str
    tooltip: str | None = None


class LiquidityLockItem(BaseModel):
    lockerName: str
    contractAddress: str
    amount: str
    unlocksIn: str
    unlockDate: str
    txHash: str


class LiquidityLocksSchema(BaseModel):
    totalLocked: str
    locks: list[LiquidityLockItem]


class AuditCommentItem(BaseModel):
    author: str
    reputation: str | None = None
    text: str


class CommunitySentimentSchema(BaseModel):
    bullish: int
    bearish: int
    upvotes: str
    watchlists: str
    comments: list[AuditCommentItem]


class VulnerabilitiesSchema(BaseModel):
    critical: int
    high: int
    medium: int
    low: int


class AuditResponse(BaseModel):
    projectIcon: str
    projectName: str
    contractAddress: str
    fullAddress: str
    network: str
    auditCompleted: str
    isVerified: bool
    verdict: str
    riskLevel: str
    trustScore: float
    trustSummary: str
    securityChecks: list[SecurityCheckItem]
    vulnerabilities: VulnerabilitiesSchema
    liquidityLocks: LiquidityLocksSchema
    communitySentiment: CommunitySentimentSchema
    tokenSymbol: str


class AuditCommentBody(BaseModel):
    text: str


class AuditCommentCreated(BaseModel):
    id: str
    author: str
    text: str
    createdAt: str


# ----- Watchlist -----
class WatchlistPostBody(BaseModel):
    contractAddress: str


# ----- Submissions -----
_EVM_ADDR = re.compile(r"^0x[0-9a-fA-F]{40}$")
# Identificadores nativos no-EVM (p. ej. Solana): longitud acotada, sin espacios.
_NON_EVM_ID = re.compile(r"^[A-Za-z0-9]{32,128}$")


def _validate_contract_address(name: str, v: str) -> str:
    v = v.strip()
    if _EVM_ADDR.match(v):
        return v
    if _NON_EVM_ID.match(v):
        return v
    raise ValueError(
        f"{name} must be EVM (0x + 40 hex) or a native contract/program id (32–128 alphanumerics)"
    )


class SubmissionPostBody(BaseModel):
    projectName: str = Field(..., min_length=1, max_length=256)
    tokenSymbol: str = Field(..., min_length=1, max_length=32)
    totalSupply: str = Field(..., min_length=1, max_length=64)
    launchSupply: str = Field(..., min_length=1, max_length=64)
    contactEmail: str = Field(..., min_length=5, max_length=256)
    contractAddress: str = Field(..., description="Contrato de presale/launch")
    network: str = Field(..., min_length=1, max_length=128)
    logoUrl: str | None = Field(None, max_length=512)
    contractTokenAddress: str | None = Field(None, description="Contrato del token si difiere del de launch")
    chainId: int | None = Field(None, ge=0, le=2_000_000_000)
    auditReport: str = Field("", max_length=512)
    twitter: str | None = None
    telegram: str | None = None

    @field_validator("contractAddress")
    @classmethod
    def contract_addr(cls, v: str) -> str:
        return _validate_contract_address("contractAddress", v)

    @field_validator("contractTokenAddress")
    @classmethod
    def token_addr(cls, v: str | None) -> str | None:
        if v is None or not str(v).strip():
            return None
        return _validate_contract_address("contractTokenAddress", v)

    @field_validator("contactEmail")
    @classmethod
    def contact_email_fmt(cls, v: str) -> str:
        v = v.strip()
        if not re.match(r"^[^@\s]+@[^@\s]+\.[^@\s]+$", v):
            raise ValueError("contactEmail must be a valid email address")
        return v


class SubmissionCreated(BaseModel):
    id: str
    status: str
    createdAt: str


class SubmissionDetailResponse(BaseModel):
    id: str | None = None
    walletAddress: str | None = None
    projectName: str | None = None
    tokenSymbol: str | None = None
    totalSupply: str | None = None
    launchSupply: str | None = None
    contactEmail: str | None = None
    logoUrl: str | None = None
    contractAddress: str
    contractTokenAddress: str | None = None
    chainId: int | None = None
    network: str
    auditReport: str
    twitter: str | None = None
    telegram: str | None = None
    status: str
    reviewedAt: str | None = None
    reviewerNotes: str | None = None
    reviewerWallet: str | None = None
    createdAt: str | None = None


class SubmissionMineItem(BaseModel):
    id: str
    projectName: str | None = None
    contractAddress: str
    network: str
    status: str
    createdAt: str


class SubmissionReviewBody(BaseModel):
    decision: Literal["approve", "reject"]
    reviewerNotes: str | None = Field(None, max_length=4000)


# ----- Paginación -----
class GemsListParams(BaseModel):
    page: int = Field(1, ge=1)
    limit: int = Field(20, ge=1, le=100)
    category: str | None = None
    search: str | None = None
    verified: bool | None = None
    rugChecked: bool | None = None
    minScore: float | None = None
    maxScore: float | None = None


class PresalesListParams(BaseModel):
    status: str | None = None
    page: int = Field(1, ge=1)
    limit: int = Field(20, ge=1, le=100)
    search: str | None = None


class ContributionsListParams(BaseModel):
    status: str | None = None
    search: str | None = None
    page: int = Field(1, ge=1)
    limit: int = Field(20, ge=1, le=100)


class PriceHistoryParams(BaseModel):
    range: Literal["24h", "7d", "30d"] = "24h"
    points: int = Field(24, ge=1, le=200)
