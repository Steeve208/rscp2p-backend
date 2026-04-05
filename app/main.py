"""
RSC P2P Backend - Punto de entrada.
API bajo /api y Socket.IO para el terminal de finanzas y Launchpad.
"""

from contextlib import asynccontextmanager
from collections import defaultdict, deque
from datetime import datetime, timezone
import logging
import time
from uuid import uuid4

import jwt
import socketio
from fastapi import APIRouter, FastAPI, Request
from fastapi.exceptions import RequestValidationError
from fastapi.responses import JSONResponse
from fastapi.middleware.cors import CORSMiddleware

from app.api.routes import auth, health, orders
from app.api.routes.deposits import router as deposits_router
from app.api.routes.escrow import router as escrow_router
from app.api.routes.users import router as users_router
from app.api.routes.terminal import alerts as terminal_alerts, market as terminal_market
from app.config import settings
from app.launchpad.routes import router as launchpad_router
from app.redis import close_redis, get_redis, redis_rate_limit
from app.services.outbox_worker import start_outbox_worker, stop_outbox_worker
from app.websocket.socketio import sio

logger = logging.getLogger("rsc-backend")
if not logger.handlers:
    handler = logging.StreamHandler()
    formatter = logging.Formatter(
        '{"timestamp":"%(asctime)s","level":"%(levelname)s","message":"%(message)s"}'
    )
    handler.setFormatter(formatter)
    logger.addHandler(handler)
logger.setLevel(getattr(logging, settings.log_level.upper(), logging.INFO))

_rate_limit_store: dict[str, deque[float]] = defaultdict(deque)
_auth_rate_limit_store: dict[str, deque[float]] = defaultdict(deque)


@asynccontextmanager
async def lifespan(app: FastAPI):
    if settings.is_production and (
        not settings.jwt_secret or settings.jwt_secret == "change-me-in-production"
    ):
        logger.error(
            "Producción activa: debe definir JWT_SECRET en el entorno (nunca use el valor por defecto)."
        )
        raise RuntimeError("JWT_SECRET requerido en producción")
    if settings.is_production and settings.cors_origins.strip() == "*":
        logger.error(
            "Producción activa: CORS_ORIGINS='*' no permitido. Defina orígenes explícitos."
        )
        raise RuntimeError("CORS_ORIGINS explícitos requeridos en producción")
    if settings.is_production and "sqlite" in settings.database_url:
        logger.critical(
            "Producción activa con SQLite — use PostgreSQL (DATABASE_URL) para concurrencia y durabilidad."
        )
    await get_redis()
    from app.launchpad.seed import init_db
    init_db()
    await start_outbox_worker()
    yield
    await stop_outbox_worker()
    await close_redis()


fastapi_app = FastAPI(
    title="RSC P2P Backend",
    description="API para terminal de finanzas RSC P2P y Launchpad",
    version="0.1.0",
    lifespan=lifespan,
)

fastapi_app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.cors_origins_list,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


@fastapi_app.middleware("http")
async def correlation_id_middleware(request: Request, call_next):
    """Attach a unique X-Request-ID to every request/response for distributed tracing."""
    request_id = request.headers.get("X-Request-ID") or str(uuid4())
    request.state.request_id = request_id
    response = await call_next(request)
    response.headers["X-Request-ID"] = request_id
    return response


@fastapi_app.middleware("http")
async def rate_limit_middleware(request: Request, call_next):
    client_ip = request.client.host if request.client else "unknown"
    path = request.url.path or ""
    now = time.time()

    is_auth = path.rstrip("/").endswith("/auth/challenge") or path.rstrip("/").endswith("/auth/verify")
    if is_auth:
        window = settings.rate_limit_auth_window_seconds
        max_requests = settings.rate_limit_auth_max_requests
    else:
        window = settings.rate_limit_window_seconds
        max_requests = settings.rate_limit_max_requests

    redis_key = f"rl:{'auth' if is_auth else 'api'}:{client_ip}"
    allowed = await redis_rate_limit(redis_key, window, max_requests)

    if not allowed:
        # Fallback: also check in-memory (Redis said no)
        return JSONResponse(
            status_code=429,
            content={
                "error": "rate_limit_exceeded",
                "message": "Too many requests",
                "windowSeconds": window,
                "maxRequests": max_requests,
            },
        )

    # In-memory fallback when Redis is unavailable
    mem_queue = (_auth_rate_limit_store if is_auth else _rate_limit_store)[client_ip]
    while mem_queue and mem_queue[0] < now - window:
        mem_queue.popleft()
    if len(mem_queue) >= max_requests:
        return JSONResponse(
            status_code=429,
            content={
                "error": "rate_limit_exceeded",
                "message": "Too many requests",
                "windowSeconds": window,
                "maxRequests": max_requests,
            },
        )
    mem_queue.append(now)

    response = await call_next(request)
    return response


@fastapi_app.middleware("http")
async def security_headers_middleware(request: Request, call_next):
    response = await call_next(request)
    if settings.strict_security_headers:
        response.headers["X-Content-Type-Options"] = "nosniff"
        response.headers["X-Frame-Options"] = "DENY"
        response.headers["Referrer-Policy"] = "strict-origin-when-cross-origin"
        response.headers["Permissions-Policy"] = "geolocation=(), microphone=(), camera=()"
    return response


def _wallet_from_authorization_header(request: Request) -> str | None:
    """Solo para auditoría; no sustituye get_current_user."""
    auth = request.headers.get("authorization")
    if not auth or not auth.lower().startswith("bearer "):
        return None
    token = auth[7:].strip()
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
        sub = payload.get("sub")
        return str(sub).strip().lower() if isinstance(sub, str) else None
    except jwt.PyJWTError:
        return None


@fastapi_app.middleware("http")
async def audit_log_middleware(request: Request, call_next):
    response = await call_next(request)
    path = request.url.path or ""
    if (
        request.method in ("POST", "PUT", "PATCH", "DELETE")
        and path.startswith("/api/")
        and not path.rstrip("/").endswith("/health")
    ):
        actor = _wallet_from_authorization_header(request)
        client_ip = request.client.host if request.client else "unknown"
        rid = getattr(request.state, "request_id", "-")
        logger.info(
            "audit method=%s path=%s status=%s actor=%s ip=%s rid=%s",
            request.method,
            path,
            response.status_code,
            actor or "-",
            client_ip,
            rid,
        )
    return response


@fastapi_app.middleware("http")
async def request_logging_middleware(request: Request, call_next):
    started = time.time()
    response = await call_next(request)
    elapsed_ms = round((time.time() - started) * 1000, 2)
    rid = getattr(request.state, "request_id", "-")
    logger.info(
        "request method=%s path=%s status=%s elapsedMs=%s rid=%s",
        request.method,
        request.url.path,
        response.status_code,
        elapsed_ms,
        rid,
    )
    return response


@fastapi_app.exception_handler(RequestValidationError)
async def validation_exception_handler(request: Request, exc: RequestValidationError):
    logger.warning("validation_error path=%s errors=%s", request.url.path, exc.errors())
    return JSONResponse(
        status_code=422,
        content={
            "error": "validation_error",
            "message": "Validation failed",
            "details": exc.errors(),
        },
    )


@fastapi_app.exception_handler(Exception)
async def unhandled_exception_handler(request: Request, exc: Exception):
    logger.exception("unhandled_exception path=%s", request.url.path)
    return JSONResponse(
        status_code=500,
        content={
            "error": "internal_server_error",
            "message": "Unexpected server error",
            "timestamp": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        },
    )

api_router = APIRouter(prefix="/api")
api_router.include_router(health.router)
api_router.include_router(auth.router)
api_router.include_router(orders.router)
api_router.include_router(escrow_router)
api_router.include_router(deposits_router)
api_router.include_router(users_router)
api_router.include_router(launchpad_router)
api_router.include_router(terminal_market.router)
api_router.include_router(terminal_alerts.router)
fastapi_app.include_router(api_router)


@fastapi_app.get("/")
def root():
    return {"service": "RSC P2P Backend", "status": "ok"}


# ASGI app para uvicorn: Socket.IO + FastAPI
app = socketio.ASGIApp(sio, other_asgi_app=fastapi_app)
