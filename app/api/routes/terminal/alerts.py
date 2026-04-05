"""
Alertas del terminal: GET /api/terminal/alerts, POST /api/terminal/alerts.
Persistentes en BD, filtradas por usuario autenticado.
Al crear una alerta se emite evento Socket.IO alert:new al room del usuario.
"""

from fastapi import APIRouter, Depends, Query
from sqlalchemy.orm import Session

from app.api.routes.auth import get_current_user
from app.db import get_db
from app.schemas.alert import Alert, AlertCreate
from app.schemas.auth import UserResponse
from app.services import alerts as alerts_service
from app.websocket.socketio import sio

router = APIRouter(prefix="/terminal/alerts", tags=["terminal-alerts"])


@router.get("", response_model=list[Alert])
def get_alerts(
    limit: int = Query(100, ge=1, le=500),
    unread_only: bool = Query(False),
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
):
    return alerts_service.list_alerts(db, user_id=user.id, limit=limit, unread_only=unread_only)


@router.post("", response_model=Alert)
async def create_alert(
    create: AlertCreate,
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
):
    alert = alerts_service.add_alert(db, user_id=user.id, create=create)
    payload = alert.model_dump(mode="json")
    await sio.emit("alert:new", payload, room=f"wallet:{user.walletAddress.lower()}")
    return alert


@router.post("/{alert_id}/read")
def mark_alert_read(
    alert_id: str,
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
):
    ok = alerts_service.mark_as_read(db, user_id=user.id, alert_id=alert_id)
    return {"ok": ok}


@router.post("/read-all")
def mark_all_alerts_read(
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
):
    alerts_service.mark_all_read(db, user_id=user.id)
    return {"ok": True}


@router.get("/unread-count")
def get_unread_count(
    db: Session = Depends(get_db),
    user: UserResponse = Depends(get_current_user),
):
    count = alerts_service.get_unread_count(db, user_id=user.id)
    return {"count": count}
