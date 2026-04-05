# Checklist — Backend Launchpad (LAUNCHPAD_ESTUDIO_BACKEND.md)

Verificación de que todo lo indicado en `docs/LAUNCHPAD_ESTUDIO_BACKEND.md` está implementado en el backend. **Sin mocks**: datos desde base de datos (SQLite + SQLAlchemy).

---

## 1. Estructura y configuración

| Requisito | Estado | Ubicación |
|-----------|--------|------------|
| Carpeta `launchpad` en el backend | ✅ | `p2p-backend/app/launchpad/` |
| Base URL API bajo `/api` | ✅ | Router con `prefix="/api"` en `main.py`, launchpad con `prefix="/launchpad"` → `/api/launchpad/*` |
| Auth compartido (challenge/verify/refresh/me/logout) | ✅ | `app/api/routes/auth.py` (existente) |
| Base de datos persistente | ✅ | `app/db.py` (engine, SessionLocal, Base); `app/config.py` → `database_url` |
| Sin datos mock | ✅ | Servicios leen/escriben solo en DB; seed inicial en `app/launchpad/seed.py` |

---

## 2. Endpoints implementados

### 2.1 Auth (compartido)

| Método | Ruta | Estado |
|--------|------|--------|
| POST | `/api/auth/challenge` | ✅ Existente |
| POST | `/api/auth/verify` | ✅ Existente |
| POST | `/api/auth/refresh` | ✅ Existente |
| GET | `/api/auth/me` | ✅ Existente |
| POST | `/api/auth/logout` | ✅ Existente |

### 2.2 Gems (Explorer, Leaderboard)

| Método | Ruta | Query / Respuesta | Estado |
|--------|------|-------------------|--------|
| GET | `/api/launchpad/gems` | `page`, `limit`, `category`, `search`, `verified`, `rugChecked`, `minScore`, `maxScore` → `{ data, total, page, limit, totalPages }` | ✅ |
| GET | `/api/launchpad/gems/featured` | `{ data: FeaturedGem }` | ✅ |
| GET | `/api/launchpad/gems/stats` | `{ data: GlobalStats }` | ✅ |
| GET | `/api/launchpad/gems/:address` | `{ data: Gem }` | ✅ |

### 2.3 Presales

| Método | Ruta | Estado |
|--------|------|--------|
| GET | `/api/launchpad/presales` | ✅ `status`, `page`, `limit`, `search` |
| GET | `/api/launchpad/presales/:id` | ✅ |
| GET | `/api/launchpad/presales/:id/contributions` | ✅ `limit` |
| POST | `/api/launchpad/presales/:id/contributions` | ✅ Auth Bearer; body `walletAddress`, `amount`, `txHash` |

### 2.4 Tokens (token/[address])

| Método | Ruta | Estado |
|--------|------|--------|
| GET | `/api/launchpad/tokens/:address` | ✅ |
| GET | `/api/launchpad/tokens/:address/price-history` | ✅ `range`, `points` |
| GET | `/api/launchpad/tokens/:address/orderbook` | ✅ |
| GET | `/api/launchpad/tokens/:address/tokenomics` | ✅ |
| GET | `/api/launchpad/tokens/:address/sentiment` | ✅ |
| POST | `/api/launchpad/tokens/:address/sentiment/vote` | ✅ Auth; body `{ vote: "bullish" \| "bearish" }` |

### 2.5 Portfolio / Contribuciones

| Método | Ruta | Estado |
|--------|------|--------|
| GET | `/api/launchpad/contributions/me` | ✅ Auth; `status`, `search`, `page`, `limit` |
| GET | `/api/launchpad/contributions/:id` | ✅ Auth |
| GET | `/api/launchpad/contributions/by-tx/:hash` | ✅ Auth |

### 2.6 Audit

| Método | Ruta | Estado |
|--------|------|--------|
| GET | `/api/launchpad/audit/:address` | ✅ |
| POST | `/api/launchpad/audit/:address/comment` | ✅ Auth; body `{ text }` |

### 2.7 Watchlist

| Método | Ruta | Estado |
|--------|------|--------|
| GET | `/api/launchpad/watchlist` | ✅ Auth |
| POST | `/api/launchpad/watchlist` | ✅ Auth; body `{ contractAddress }` |
| DELETE | `/api/launchpad/watchlist/:address` | ✅ Auth |

### 2.8 Submissions

| Método | Ruta | Estado |
|--------|------|--------|
| POST | `/api/launchpad/submissions` | ✅ Auth; body `contractAddress`, `network`, `auditReport`, `twitter?`, `telegram?` |
| GET | `/api/launchpad/submissions/:id` | ✅ Auth |
| GET | `/api/launchpad/submissions/mine` | ✅ Auth; `page`, `limit` |

---

## 3. WebSocket (Live Participation Feed)

| Requisito | Estado | Ubicación |
|-----------|--------|-----------|
| Cliente → Servidor: `presale:subscribe` con `{ presaleId }` | ✅ | `app/websocket/socketio.py` → `@sio.on("presale:subscribe")` |
| Cliente → Servidor: `presale:unsubscribe` con `{ presaleId }` | ✅ | `@sio.on("presale:unsubscribe")` |
| Servidor → Cliente: `presale:contribution` con `{ id, walletAddress, amount, timestamp }` | ✅ | Cola + consumer; `notify_presale_contribution()` llamada desde POST presales/:id/contributions |

---

## 4. Tipos de datos (schemas)

| Tipo (doc §5) | Estado | Ubicación |
|---------------|--------|-----------|
| Gem (5.1) | ✅ | `app/launchpad/schemas.py` → `GemResponse` |
| FeaturedGem (5.2) | ✅ | `FeaturedGemResponse`, `FeaturedParticipant` |
| GlobalStats (5.3) | ✅ | `GlobalStatsResponse` |
| Presale (5.4) | ✅ | `PresaleResponse`, `VestingTerms` |
| PresaleContributionItem / PostPresaleContributionBody | ✅ | En schemas |
| Contribution (5.5) | ✅ | `ContributionResponse` |
| TokenDetail (5.6) | ✅ | `TokenDetailResponse`, `TokenomicsSchema`, `DaoSentimentSchema`, `SentimentComment` |
| PriceHistoryPoint, OrderBookResponse, TokenomicsResponse, SentimentResponse, SentimentVoteBody | ✅ | En schemas |
| Audit (5.7) | ✅ | `AuditResponse`, `SecurityCheckItem`, `LiquidityLocksSchema`, `CommunitySentimentSchema`, etc. |
| AuditCommentBody, AuditCommentCreated | ✅ | En schemas |
| WatchlistPostBody, SubmissionPostBody, SubmissionCreated, SubmissionDetailResponse | ✅ | En schemas |

---

## 5. Formato de respuestas y errores

| Requisito | Estado |
|-----------|--------|
| Éxito con datos: `{ "data": T }` | ✅ Todas las rutas devuelven `{"data": ...}` |
| Listas: `{ "data", "total", "page", "limit", "totalPages" }` | ✅ `_paginated()` en routes |
| Error: HTTPException (FastAPI usa `detail`; front puede mapear a `message`) | ✅ |
| Códigos: 200/201, 400, 401, 403, 404, 500 | ✅ Uso estándar de FastAPI |

---

## 6. Base de datos y seed

| Requisito | Estado | Ubicación |
|-----------|--------|-----------|
| Modelos: Gem, Presale, Contribution, TokenInfo, TokenSentiment, SentimentVote, PriceHistory, OrderBookEntry, Audit, AuditComment, Submission, Watchlist | ✅ | `app/launchpad/models.py` |
| Creación de tablas al arranque | ✅ | `app/launchpad/seed.init_db()` en lifespan de `main.py` |
| Seed inicial (al menos 1 gem featured, 1 presale, 1 audit, token info, orderbook, price history) | ✅ | `app/launchpad/seed.run_seed()`; se ejecuta si no hay gems |

---

## 7. Archivos creados/modificados

| Archivo | Descripción |
|---------|-------------|
| `app/launchpad/__init__.py` | Exporta `launchpad_router` |
| `app/launchpad/models.py` | Modelos SQLAlchemy |
| `app/launchpad/schemas.py` | Schemas Pydantic (request/response) |
| `app/launchpad/service.py` | Lógica de negocio y CRUD (sin mocks) |
| `app/launchpad/routes.py` | Rutas FastAPI bajo `/launchpad` |
| `app/launchpad/seed.py` | Seed inicial y `init_db()` |
| `app/db.py` | Engine, SessionLocal, Base, get_db |
| `app/config.py` | Añadido `database_url` |
| `app/websocket/socketio.py` | Eventos `presale:subscribe`, `presale:unsubscribe`, cola y emisión `presale:contribution`, `notify_presale_contribution()` |
| `app/main.py` | Inclusión de `launchpad_router`, lifespan con `init_db()` |
| `requirements.txt` | Añadido `sqlalchemy>=2.0.0` |

---

## 8. Checklist mínimo producción (doc §9)

| Ítem | Estado |
|------|--------|
| GET /launchpad/gems (paginación + filtros) | ✅ |
| GET /launchpad/gems/featured | ✅ |
| GET /launchpad/gems/stats | ✅ |
| GET /launchpad/presales/:id | ✅ |
| GET /launchpad/presales/:id/contributions | ✅ |
| POST /launchpad/submissions (con JWT) | ✅ |
| WebSocket: presale:subscribe, presale:unsubscribe, presale:contribution | ✅ |
| GET /launchpad/contributions/by-tx/:hash | ✅ |
| GET /launchpad/contributions/me | ✅ |
| GET /launchpad/tokens/:address, price-history, orderbook | ✅ |
| GET /launchpad/audit/:address | ✅ |

---

## 9. Cómo ejecutar y probar

1. **Instalar dependencias**  
   `pip install -r requirements.txt` (incluye `sqlalchemy>=2.0.0`).

2. **Arrancar el backend**  
   Desde la raíz del backend:  
   `uvicorn app.main:app --host 0.0.0.0 --port 8000`  
   (o usar `app.main:fastapi_app` si se monta Socket.IO por encima).

   El ASGI app completo es `app.main:app` (Socket.IO + FastAPI). Al arrancar se ejecuta `init_db()`: se crean las tablas y se ejecuta el seed si la tabla de gems está vacía.

3. **Probar endpoints**  
   - `GET http://localhost:8000/api/launchpad/gems`  
   - `GET http://localhost:8000/api/launchpad/gems/featured`  
   - `GET http://localhost:8000/api/launchpad/gems/stats`  
   - `GET http://localhost:8000/api/launchpad/presales`  
   - `GET http://localhost:8000/api/launchpad/tokens/0xaether12345678901234567890123456789012` (dirección del seed)  
   - `GET http://localhost:8000/api/launchpad/audit/0xaether12345678901234567890123456789012`  

4. **WebSocket**  
   Conectar al mismo host/puerto y emitir `presale:subscribe` con `{ presaleId: "<id>" }`. Tras hacer POST a `/api/launchpad/presales/:id/contributions`, los clientes suscritos a esa room reciben `presale:contribution`.

---

**Resumen**: Todo lo indicado en `docs/LAUNCHPAD_ESTUDIO_BACKEND.md` está cubierto en el backend: carpeta `launchpad`, endpoints, WebSocket, tipos, formato de respuestas, DB persistente y seed. No se usan mocks; los datos provienen de la base de datos.
