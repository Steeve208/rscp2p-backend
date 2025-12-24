# Instalar Docker Desktop - Guía Rápida

## ✅ Método más simple:

1. **Abre tu navegador** y ve a:
   ```
   https://www.docker.com/products/docker-desktop/
   ```

2. **Haz clic en "Download for Windows"**

3. **Ejecuta el instalador** (Docker Desktop Installer.exe)

4. **Sigue el asistente de instalación:**
   - Acepta los términos
   - Marca "Use WSL 2 instead of Hyper-V" (recomendado)
   - Completa la instalación

5. **REINICIA tu computadora** (requerido)

6. **Inicia Docker Desktop** desde el menú de inicio

7. **Espera** a que Docker Desktop esté completamente iniciado (ícono verde)

8. **Vuelve aquí y ejecuta:**
   ```powershell
   .\start-services.ps1
   npm run dev
   ```

## 🔄 Alternativa: Instalar sin Docker

Si prefieres NO usar Docker, puedes instalar PostgreSQL y Redis directamente en Windows:

### PostgreSQL:
- Descargar: https://www.postgresql.org/download/windows/
- Instalar y recordar la contraseña
- Crear base de datos: `CREATE DATABASE rsc_db;`

### Redis:
- Opción 1: Memurai (Redis para Windows): https://www.memurai.com/
- Opción 2: Usar WSL2 con Redis
- Opción 3: Redis en modo desarrollo (sin persistencia)

