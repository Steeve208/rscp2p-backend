"""
Socket.IO server: eventos order:*, presale:* (alineado con frontend).
Órdenes: solo emisión vía notify_order_updated (cambios reales), sin broadcast aleatorio.
Chat: JWT obligatorio en conexión; usuario solo desde sesión; rate limit por sid/presale.
"""

import asyncio
import logging
import queue
import time
import urllib.parse

import jwt
import socketio

from app.config import settings
from app.db import SessionLocal

logger = logging.getLogger("rsc-backend")

_client_manager = None
try:
    if settings.redis_url:
        _client_manager = socketio.AsyncRedisManager(settings.redis_url)
        logger.info("Socket.IO using Redis adapter: %s", settings.redis_url)
except Exception as exc:
    logger.warning("Socket.IO Redis adapter unavailable, using in-memory: %s", exc)
    _client_manager = None

sio = socketio.AsyncServer(
    async_mode="asgi",
    cors_allowed_origins=settings.cors_origins_list,
    client_manager=_client_manager,
)

_launchpad_queue: queue.Queue = queue.Queue()
_order_events_queue: queue.Queue = queue.Queue()
_launchpad_consumer_task: asyncio.Task | None = None
_order_events_consumer_task: asyncio.Task | None = None

# Chat rate limit: (sid, presale_id) -> list of timestamps (last 60s). Max CHAT_RATE_PER_MIN per room.
_chat_rates: dict[tuple[str, str], list[float]] = {}
CHAT_RATE_PER_MIN = 10
CHAT_RATE_WINDOW = 60.0


def _token_from_connect(environ: dict, auth: dict | None) -> str | None:
    if auth and isinstance(auth, dict):
        t = auth.get("token")
        if t:
            return str(t).strip()
    qs = environ.get("QUERY_STRING") or ""
    if not qs:
        return None
    params = urllib.parse.parse_qs(qs)
    tokens = params.get("token") or []
    return tokens[0].strip() if tokens else None


def _chat_rate_limit(sid: str, presale_id: str) -> bool:
    """True if under limit, False if over (should reject)."""
    key = (sid, str(presale_id))
    now = time.time()
    if key not in _chat_rates:
        _chat_rates[key] = []
    times = _chat_rates[key]
    times.append(now)
    times[:] = [t for t in times if now - t < CHAT_RATE_WINDOW]
    return len(times) <= CHAT_RATE_PER_MIN


async def _launchpad_consumer_loop() -> None:
    """Consume cola de contribuciones y emite presale:contribution a la room del presale."""
    while True:
        try:
            presale_id, payload = _launchpad_queue.get_nowait()
            room = f"presale:{presale_id}"
            await sio.emit("presale:contribution", payload, room=room)
        except queue.Empty:
            await asyncio.sleep(0.2)


def _public_order_summary(payload: dict) -> dict:
    """Sanitized payload for the public marketplace room (no full wallet addresses)."""
    return {
        "id": payload.get("id"),
        "cryptoCurrency": payload.get("cryptoCurrency"),
        "cryptoAmount": payload.get("cryptoAmount"),
        "fiatCurrency": payload.get("fiatCurrency"),
        "fiatAmount": payload.get("fiatAmount"),
        "pricePerUnit": payload.get("pricePerUnit"),
        "paymentMethod": payload.get("paymentMethod"),
        "status": payload.get("status"),
        "createdAt": payload.get("createdAt"),
    }


async def _order_events_consumer_loop() -> None:
    """Emite order:created / order:updated a rooms específicos por wallet."""
    while True:
        try:
            event, payload, seller_id, buyer_id = _order_events_queue.get_nowait()
            if event == "order:created":
                await sio.emit(event, _public_order_summary(payload), room="marketplace")
            if seller_id:
                await sio.emit(event, payload, room=f"wallet:{seller_id}")
            if buyer_id:
                await sio.emit(event, payload, room=f"wallet:{buyer_id}")
        except queue.Empty:
            await asyncio.sleep(0.05)


def notify_presale_contribution(presale_id: str, payload: dict) -> None:
    """Llamar desde API al crear una contribución; emite por WebSocket a suscriptores."""
    _launchpad_queue.put((presale_id, payload))


def notify_order_updated(
    order_payload: dict,
    event: str = "order:updated",
    seller_id: str | None = None,
    buyer_id: str | None = None,
) -> None:
    """Emitir evento de orden solo cuando hay cambio real (llamar desde orders/escrow tras commit)."""
    _order_events_queue.put((event, order_payload, seller_id, buyer_id))


@sio.event
async def connect(sid: str, environ: dict, auth: dict | None = None) -> None:
    global _launchpad_consumer_task, _order_events_consumer_task
    token = _token_from_connect(environ, auth)
    wallet = None
    if token:
        try:
            payload = jwt.decode(
                token,
                settings.jwt_secret,
                algorithms=[settings.jwt_algorithm],
            )
            if payload.get("type") == "access" and payload.get("sub"):
                wallet = str(payload["sub"]).strip().lower()
        except Exception:
            pass
    await sio.save_session(sid, {"wallet": wallet})
    if wallet:
        await sio.enter_room(sid, f"wallet:{wallet}")
    if _launchpad_consumer_task is None or _launchpad_consumer_task.done():
        _launchpad_consumer_task = asyncio.create_task(_launchpad_consumer_loop())
    if _order_events_consumer_task is None or _order_events_consumer_task.done():
        _order_events_consumer_task = asyncio.create_task(_order_events_consumer_loop())


@sio.event
async def disconnect(sid: str) -> None:
    pass


@sio.event
async def subscribe(sid: str) -> None:
    """Frontend emite 'subscribe' al conectar — joins the public marketplace room."""
    await sio.enter_room(sid, "marketplace")


@sio.on("presale:subscribe")
async def presale_subscribe(sid: str, data: dict) -> None:
    """Cliente se suscribe al feed de un presale. data = { presaleId }."""
    presale_id = (data or {}).get("presaleId") or (data or {}).get("presale_id")
    if presale_id:
        await sio.enter_room(sid, f"presale:{presale_id}")


@sio.on("presale:unsubscribe")
async def presale_unsubscribe(sid: str, data: dict) -> None:
    """Cliente se desuscribe. data = { presaleId }."""
    presale_id = (data or {}).get("presaleId") or (data or {}).get("presale_id")
    if presale_id:
        await sio.leave_room(sid, f"presale:{presale_id}")


# ----- Presale Chat (room: presale-chat:{presaleId}) -----

@sio.on("presale:chat:join")
async def presale_chat_join(sid: str, data: dict) -> None:
    """Cliente entra a la sala de chat del presale. data = { presaleId }."""
    presale_id = (data or {}).get("presaleId") or (data or {}).get("presale_id")
    if presale_id:
        await sio.enter_room(sid, f"presale-chat:{presale_id}")


@sio.on("presale:chat:leave")
async def presale_chat_leave(sid: str, data: dict) -> None:
    """Cliente sale de la sala de chat. data = { presaleId }."""
    presale_id = (data or {}).get("presaleId") or (data or {}).get("presale_id")
    if presale_id:
        await sio.leave_room(sid, f"presale-chat:{presale_id}")


@sio.on("presale:chat:message")
async def presale_chat_message(sid: str, data: dict) -> None:
    """Chat: user solo desde sesión JWT. Sin token válido no se envía ni persiste."""
    payload = data or {}
    presale_id = payload.get("presaleId") or payload.get("presale_id")
    message = (payload.get("message") or "").strip()
    if not presale_id or not message:
        return
    session = await sio.get_session(sid)
    user = session.get("wallet") if session else None
    if not user:
        await sio.emit("presale:chat:error", {"detail": "auth_required"}, room=sid)
        return
    if not _chat_rate_limit(sid, str(presale_id)):
        await sio.emit("presale:chat:error", {"detail": "rate_limit"}, room=sid)
        return
    from datetime import datetime, timezone

    timestamp = datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")
    out = {
        "presaleId": str(presale_id),
        "user": user,
        "message": message,
        "timestamp": timestamp,
    }
    try:
        from sqlalchemy import select

        from app.launchpad.models import PresaleChatMessageModel, PresaleModel

        db = SessionLocal()
        try:
            presale = db.execute(select(PresaleModel).where(PresaleModel.id == presale_id)).scalars().first()
            if not presale:
                presale = db.execute(
                    select(PresaleModel).where(PresaleModel.contract_address == presale_id)
                ).scalars().first()
            if presale:
                msg = PresaleChatMessageModel(
                    presale_id=presale.id,
                    user_id=user,
                    message=message,
                )
                db.add(msg)
                db.commit()
        finally:
            db.close()
    except Exception as exc:
        logger.warning("chat persist error presale=%s user=%s: %s", presale_id, user, exc)
        await sio.emit("presale:chat:error", {"detail": "persist_failed"}, room=sid)
    room = f"presale-chat:{presale_id}"
    await sio.emit("presale:chat:message", out, room=room)
