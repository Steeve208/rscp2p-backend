"""
Outbox consumer worker.
Polls outbox_events for unprocessed rows and marks them as processed.
Runs as an asyncio background task started from the application lifespan.
"""

import asyncio
import json
import logging
from datetime import datetime, timezone

from sqlalchemy import select, update
from sqlalchemy.orm import Session

from app.db import SessionLocal
from app.models.outbox import OutboxEventModel

logger = logging.getLogger("rsc-backend")

POLL_INTERVAL_SECONDS = 1.0
BATCH_SIZE = 50


def _process_event(event: OutboxEventModel) -> None:
    """
    Process a single outbox event. Extend this to publish to external systems
    (message queue, webhook, etc.). Currently logs and marks as processed.
    """
    try:
        payload = json.loads(event.payload) if isinstance(event.payload, str) else event.payload
    except (json.JSONDecodeError, TypeError):
        payload = {}

    logger.info(
        "outbox event_type=%s aggregate=%s:%s id=%s",
        event.event_type,
        event.aggregate_type,
        event.aggregate_id,
        event.id,
    )


async def _outbox_poll_loop() -> None:
    """Continuously poll outbox_events and process unprocessed rows."""
    while True:
        try:
            db: Session = SessionLocal()
            try:
                rows = list(
                    db.scalars(
                        select(OutboxEventModel)
                        .where(OutboxEventModel.processed_at.is_(None))
                        .order_by(OutboxEventModel.created_at.asc())
                        .limit(BATCH_SIZE)
                    ).all()
                )
                if rows:
                    for row in rows:
                        _process_event(row)
                    ids = [r.id for r in rows]
                    db.execute(
                        update(OutboxEventModel)
                        .where(OutboxEventModel.id.in_(ids))
                        .values(processed_at=datetime.now(timezone.utc))
                    )
                    db.commit()
                    logger.debug("outbox processed %d events", len(rows))
            finally:
                db.close()
        except Exception as exc:
            logger.warning("outbox worker error: %s", exc)

        await asyncio.sleep(POLL_INTERVAL_SECONDS)


_outbox_task: asyncio.Task | None = None


async def start_outbox_worker() -> None:
    """Start the outbox consumer as a background task (idempotent)."""
    global _outbox_task
    if _outbox_task is not None and not _outbox_task.done():
        return
    _outbox_task = asyncio.create_task(_outbox_poll_loop())
    logger.info("Outbox worker started")


async def stop_outbox_worker() -> None:
    """Cancel the outbox consumer task."""
    global _outbox_task
    if _outbox_task is not None and not _outbox_task.done():
        _outbox_task.cancel()
        try:
            await _outbox_task
        except asyncio.CancelledError:
            pass
    _outbox_task = None
    logger.info("Outbox worker stopped")
