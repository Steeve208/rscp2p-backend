"""
Servicio de alertas persistentes por usuario (SQLAlchemy).
"""

import json
from uuid import uuid4

from sqlalchemy import func, select, update
from sqlalchemy.orm import Session

from app.models.alerts import AlertModel
from app.schemas.alert import Alert, AlertCreate


def _model_to_alert(m: AlertModel) -> Alert:
    data = None
    if m.data:
        try:
            data = json.loads(m.data)
        except (json.JSONDecodeError, TypeError):
            pass
    return Alert(
        id=m.id,
        type=m.type,
        title=m.title,
        message=m.message,
        severity=m.severity,
        timestamp=int(m.created_at.timestamp() * 1000) if m.created_at else 0,
        read=m.read,
        data=data,
    )


def list_alerts(db: Session, user_id: str, limit: int = 100, unread_only: bool = False) -> list[Alert]:
    q = select(AlertModel).where(AlertModel.user_id == user_id).order_by(AlertModel.created_at.desc()).limit(limit)
    if unread_only:
        q = q.where(AlertModel.read == False)  # noqa: E712
    rows = db.scalars(q).all()
    return [_model_to_alert(r) for r in rows]


def add_alert(db: Session, user_id: str, create: AlertCreate) -> Alert:
    data_json = json.dumps(create.data) if create.data else None
    m = AlertModel(
        id=str(uuid4()),
        user_id=user_id,
        type=create.type,
        title=create.title,
        message=create.message,
        severity=create.severity,
        data=data_json,
        read=False,
    )
    db.add(m)
    db.commit()
    db.refresh(m)
    return _model_to_alert(m)


def mark_as_read(db: Session, user_id: str, alert_id: str) -> bool:
    result = db.execute(
        update(AlertModel)
        .where(AlertModel.id == alert_id, AlertModel.user_id == user_id)
        .values(read=True)
    )
    db.commit()
    return result.rowcount > 0


def mark_all_read(db: Session, user_id: str) -> None:
    db.execute(
        update(AlertModel)
        .where(AlertModel.user_id == user_id, AlertModel.read == False)  # noqa: E712
        .values(read=True)
    )
    db.commit()


def get_unread_count(db: Session, user_id: str) -> int:
    return db.scalar(
        select(func.count()).select_from(AlertModel).where(
            AlertModel.user_id == user_id,
            AlertModel.read == False,  # noqa: E712
        )
    ) or 0
