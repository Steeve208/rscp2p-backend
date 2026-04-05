"""
Modelos SQLAlchemy para Launchpad.
Persistencia en DB: gems, presales, contributions, tokens, audit, submissions, watchlist.
"""

from datetime import datetime, timezone
from uuid import uuid4

from sqlalchemy import Boolean, Column, DateTime, Float, ForeignKey, Integer, String, Text, UniqueConstraint
from sqlalchemy.dialects.sqlite import JSON
from sqlalchemy.orm import relationship

from app.db import Base


def _utc_now():
    return datetime.now(timezone.utc)


def _uuid():
    return str(uuid4())


class GemModel(Base):
    __tablename__ = "launchpad_gems"

    id = Column(String(36), primary_key=True, default=_uuid)
    project_icon = Column(String(32))
    project_name = Column(String(256))
    description = Column(Text())
    security_score = Column(Float, default=0)
    price_change = Column(Float, default=0)
    liquidity_depth = Column(String(64))
    upvotes = Column(String(32))
    launch_date = Column(String(64))
    sparkline_data = Column(JSON)
    contract_address = Column(String(66), unique=True, index=True)
    category = Column(String(64))
    is_verified = Column(Boolean, default=False)
    rug_checked = Column(Boolean, default=False)
    price = Column(Float)
    volume_24h = Column(Float)
    is_featured = Column(Boolean, default=False)
    created_at = Column(DateTime, default=_utc_now)
    updated_at = Column(DateTime, default=_utc_now, onupdate=_utc_now)

    presale = relationship("PresaleModel", back_populates="gem", uselist=False)
    token_info = relationship("TokenInfoModel", back_populates="gem", uselist=False)


class PresaleModel(Base):
    __tablename__ = "launchpad_presales"

    id = Column(String(36), primary_key=True, default=_uuid)
    gem_id = Column(String(36), ForeignKey("launchpad_gems.id"), nullable=True)
    contract_address = Column(String(66), unique=True, index=True)
    project_name = Column(String(256))
    project_description = Column(Text())
    project_icon = Column(String(32))
    is_verified = Column(Boolean, default=False)
    token_symbol = Column(String(32))
    exchange_rate = Column(Float)
    min_buy = Column(String(32))
    max_buy = Column(String(32))
    end_date = Column(String(64))
    soft_cap = Column(String(32))
    hard_cap = Column(String(32))
    min_contrib = Column(String(32))
    max_contrib = Column(String(32))
    vesting_tge = Column(String(32))
    vesting_cliff = Column(String(32))
    vesting_linear = Column(String(32))
    vesting_total_months = Column(Integer)
    audit_url = Column(String(512))
    contract_url = Column(String(512))
    status = Column(String(32), default="active", nullable=False, index=True)
    created_at = Column(DateTime, default=_utc_now)
    updated_at = Column(DateTime, default=_utc_now, onupdate=_utc_now)

    gem = relationship("GemModel", back_populates="presale")
    contributions = relationship("ContributionModel", back_populates="presale")


class ContributionModel(Base):
    __tablename__ = "launchpad_contributions"
    __table_args__ = (UniqueConstraint("presale_id", "tx_hash", name="uq_contributions_presale_tx"),)

    id = Column(String(36), primary_key=True, default=_uuid)
    presale_id = Column(String(36), ForeignKey("launchpad_presales.id"))
    wallet_address = Column(String(66))
    amount = Column(String(32))
    tx_hash = Column(String(66))
    buy_price = Column(String(32))
    current_value = Column(String(32))
    growth = Column(String(64))
    is_loss = Column(Boolean, default=False)
    vesting_progress = Column(Float, default=0)
    next_unlock = Column(String(128))
    claimable_amount = Column(String(32))
    status = Column(String(32))
    created_at = Column(DateTime, default=_utc_now)

    presale = relationship("PresaleModel", back_populates="contributions")


class TokenInfoModel(Base):
    __tablename__ = "launchpad_token_info"

    id = Column(String(36), primary_key=True, default=_uuid)
    gem_id = Column(String(36), ForeignKey("launchpad_gems.id"), unique=True)
    contract_address = Column(String(66), unique=True, index=True)
    total_supply = Column(String(64))
    burned = Column(String(64))
    dev_wallet_lock_days = Column(Integer, default=0)
    created_at = Column(DateTime, default=_utc_now)

    gem = relationship("GemModel", back_populates="token_info")
    sentiment = relationship("TokenSentimentModel", back_populates="token_info", uselist=False)


class TokenSentimentModel(Base):
    __tablename__ = "launchpad_token_sentiment"

    id = Column(String(36), primary_key=True, default=_uuid)
    token_info_id = Column(String(36), ForeignKey("launchpad_token_info.id"), unique=True)
    score = Column(Float, default=0)
    label = Column(String(64))
    comments = Column(JSON)
    bullish_count = Column(Integer, default=0)
    bearish_count = Column(Integer, default=0)
    updated_at = Column(DateTime, default=_utc_now, onupdate=_utc_now)

    token_info = relationship("TokenInfoModel", back_populates="sentiment")


class SentimentVoteModel(Base):
    __tablename__ = "launchpad_sentiment_votes"
    __table_args__ = (UniqueConstraint("contract_address", "wallet_address", name="uq_sentiment_vote_token_wallet"),)

    id = Column(String(36), primary_key=True, default=_uuid)
    contract_address = Column(String(66))
    wallet_address = Column(String(66))
    vote = Column(String(16))
    created_at = Column(DateTime, default=_utc_now)


class PriceHistoryModel(Base):
    __tablename__ = "launchpad_price_history"

    id = Column(String(36), primary_key=True, default=_uuid)
    contract_address = Column(String(66), index=True)
    time = Column(String(64))
    price = Column(Float)
    volume = Column(Float)
    created_at = Column(DateTime, default=_utc_now)


class OrderBookEntryModel(Base):
    __tablename__ = "launchpad_orderbook"

    id = Column(String(36), primary_key=True, default=_uuid)
    contract_address = Column(String(66), index=True)
    side = Column(String(8))
    price = Column(String(32))
    amount = Column(String(32))
    score = Column(Float)
    address = Column(String(66))
    created_at = Column(DateTime, default=_utc_now)


class AuditModel(Base):
    __tablename__ = "launchpad_audits"

    id = Column(String(36), primary_key=True, default=_uuid)
    contract_address = Column(String(66), unique=True, index=True)
    project_icon = Column(String(32))
    project_name = Column(String(256))
    full_address = Column(String(66))
    network = Column(String(64))
    audit_completed = Column(String(64))
    is_verified = Column(Boolean, default=False)
    verdict = Column(String(128))
    risk_level = Column(String(64))
    trust_score = Column(Float)
    trust_summary = Column(Text())
    security_checks = Column(JSON)
    vulnerabilities = Column(JSON)
    liquidity_locks = Column(JSON)
    community_sentiment = Column(JSON)
    token_symbol = Column(String(32))
    created_at = Column(DateTime, default=_utc_now)
    updated_at = Column(DateTime, default=_utc_now, onupdate=_utc_now)

    comments_rel = relationship("AuditCommentModel", back_populates="audit")


class AuditCommentModel(Base):
    __tablename__ = "launchpad_audit_comments"

    id = Column(String(36), primary_key=True, default=_uuid)
    audit_id = Column(String(36), ForeignKey("launchpad_audits.id"))
    author = Column(String(66))
    text = Column(Text())
    created_at = Column(DateTime, default=_utc_now)

    audit = relationship("AuditModel", back_populates="comments_rel")


class SubmissionModel(Base):
    __tablename__ = "launchpad_submissions"

    id = Column(String(36), primary_key=True, default=_uuid)
    wallet_address = Column(String(66))
    # Contrato principal de presale/launch (EVM 0x… u otro formato nativo de la red).
    contract_address = Column(String(128))
    # Contrato del token (opcional).
    contract_token_address = Column(String(128), nullable=True)
    network = Column(String(64))
    chain_id = Column(Integer, nullable=True)
    project_name = Column(String(256))
    token_symbol = Column(String(32))
    total_supply = Column(String(64))
    launch_supply = Column(String(64))
    logo_url = Column(String(512), nullable=True)
    contact_email = Column(String(256))
    audit_report = Column(String(512))
    twitter = Column(String(256))
    telegram = Column(String(256))
    status = Column(String(32))
    reviewed_at = Column(DateTime, nullable=True)
    reviewer_notes = Column(Text())
    reviewer_wallet = Column(String(66), nullable=True)
    created_at = Column(DateTime, default=_utc_now)


class PresaleChatMessageModel(Base):
    __tablename__ = "launchpad_presale_chat_messages"

    id = Column(String(36), primary_key=True, default=_uuid)
    presale_id = Column(String(36), ForeignKey("launchpad_presales.id"), nullable=False, index=True)
    user_id = Column(String(66), nullable=False, index=True)  # wallet_address as user id
    message = Column(Text(), nullable=False)
    created_at = Column(DateTime, default=_utc_now)


class WatchlistModel(Base):
    __tablename__ = "launchpad_watchlist"

    __table_args__ = (UniqueConstraint("wallet_address", "contract_address", name="uq_watchlist_user_token"),)

    id = Column(String(36), primary_key=True, default=_uuid)
    wallet_address = Column(String(66), index=True)
    contract_address = Column(String(66), index=True)
    created_at = Column(DateTime, default=_utc_now)
