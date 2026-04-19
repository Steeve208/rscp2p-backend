# RSC P2P Backend (Python)

Backend en Python para el terminal de finanzas RSC P2P. API REST y WebSocket consumida por la web (Next.js en Vercel).

## Requisitos

- Python 3.10+

## Si en Windows dice "Python was not found"

Windows puede estar enviando `python` a la Microsoft Store. Haz esto:

1. **Desactiva el alias de la Store:**  
   **Configuración** → **Aplicaciones** → **Aplicaciones y características** → **Opciones avanzadas** (o **Configuración de aplicaciones avanzada**) → **Alias de ejecución de aplicaciones**.  
   Desactiva **python.exe** y **python3.exe** (así se usará el Python instalado en el equipo).

2. **Comprueba si Python ya está instalado** (por ejemplo con winget):
   ```powershell
   & "$env:LOCALAPPDATA\Programs\Python\Python312\python.exe" --version
   ```
   Si ese archivo no existe, prueba:
   ```powershell
   & "C:\Program Files\Python312\python.exe" --version
   ```
   Si uno de los dos responde con la versión, ya tienes Python; solo hace falta que el PATH o el alias estén bien (paso 1).

3. **Si no tienes Python:** instálalo desde [python.org](https://www.python.org/downloads/) y en el instalador marca **"Add python.exe to PATH"**.

Luego abre una **terminal nueva** y ejecuta `python --version`. Cuando funcione, sigue con la instalación del backend abajo.

## Instalación

**Windows (PowerShell):**
```powershell
cd p2p-backend
.\install.ps1
```

**Manual (con Python 3.10+ en PATH):**
```bash
cd p2p-backend
python -m venv .venv
.venv\Scripts\activate   # Windows
# source .venv/bin/activate   # Linux/macOS
pip install -r requirements.txt
```

## Ejecución

```powershell
.\run.ps1
```

(`run.ps1` arranca **sin** `--reload` para evitar en Windows `PermissionError: [WinError 5] Acceso denegado` en `CreateNamedPipe` del reloader nativo de uvicorn.)

Recarga al guardar cambios en código:

```powershell
.\run-reload.ps1
```

En **Windows**, `run-reload.ps1` usa **watchdog** (`watchmedo`): reinicia uvicorn al cambiar `.py` bajo `app/`, sin el reloader multiproceso de uvicorn. En Linux/macOS sigue usando `uvicorn --reload`.

O manualmente:
```bash
.\.venv\Scripts\activate
uvicorn app.main:app --host 0.0.0.0 --port 8000
```

- API base: http://localhost:8000/api

## Si aparece "ModuleNotFoundError: No module named 'sqlalchemy'"

Las dependencias no están instaladas o están incompletas. En PowerShell, desde `p2p-backend`:

```powershell
.\.venv\Scripts\Activate.ps1
pip install -r requirements.txt
```

Luego vuelve a ejecutar `.\run.ps1`.

## Si aún ves PermissionError / CreateNamedPipe

No uses **`uvicorn ... --reload`** a mano en Windows. Arranca con **`.\run.ps1`** o **`.\run-reload.ps1`** (este último ya evita ese reloader en Windows). Si quedó un proceso colgado: **`.\stop.ps1`**.

(`run-no-reload.ps1` es equivalente a `run.ps1`.)

- Docs: http://localhost:8000/docs
- Health: GET /api/health
- Órdenes: GET /api/orders, GET /api/orders/:id
- **Auth (Face ID)**: POST /api/auth/challenge, POST /api/auth/verify, POST /api/auth/refresh, GET /api/auth/me, POST /api/auth/logout
- **Mercado (OHLC)**: GET /api/terminal/market/prices?symbol=BTCUSDT&timeframe=1h
- **Alertas**: GET /api/terminal/alerts, POST /api/terminal/alerts (emite evento Socket.IO `alert:new`)
- Socket.IO: mismo host (puerto 8000), eventos `order:created`, `order:updated`, `alert:new`

Para que el frontend use este backend en local: `NEXT_PUBLIC_API_URL=http://localhost:8000/api` y `NEXT_PUBLIC_WS_URL=http://localhost:8000`. En producción define `JWT_SECRET` en `.env`.

## Estructura

```
p2p-backend/
  app/
    main.py              # FastAPI + Socket.IO ASGI
    config.py            # HOST, PORT, CORS, JWT_*
    api/routes/          # health, auth, orders
    api/routes/terminal/ # market (OHLC), alerts
    schemas/             # order, common, auth, alert, market
    services/            # orders, auth, market, alerts
    websocket/           # socketio (order:*, alert:new)
  requirements.txt
  .env.example
  install.ps1, run.ps1, run-reload.ps1, run-no-reload.ps1
  README.md
```

_(Commit de prueba de despliegue / CI — ignorar si no aplica.)_
