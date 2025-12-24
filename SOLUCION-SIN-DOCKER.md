# Solución: Instalar PostgreSQL y Redis sin Docker

Tu versión de Windows (10.0.18362) no es compatible con Docker Desktop, que requiere Windows 10 22H2 o superior.

## ✅ Solución: Instalar PostgreSQL y Redis directamente

### 1. Instalar PostgreSQL

**Opción A: Instalador oficial (Recomendado)**
1. Descarga PostgreSQL desde: https://www.postgresql.org/download/windows/
2. Ejecuta el instalador
3. Durante la instalación:
   - Usuario: `postgres`
   - Contraseña: `postgres` (o la que prefieras, actualiza `.env`)
   - Puerto: `5432` (por defecto)
4. Al final, marca "Stack Builder" si quieres herramientas adicionales

**Opción B: Usar Chocolatey (si lo tienes)**
```powershell
choco install postgresql -y
```

**Después de instalar, crear la base de datos:**
```powershell
# Abre pgAdmin o usa psql desde la línea de comandos
psql -U postgres
CREATE DATABASE rsc_db;
\q
```

### 2. Instalar Redis

**Opción A: Memurai (Redis para Windows) - RECOMENDADO**
1. Descarga desde: https://www.memurai.com/get-memurai
2. Instala Memurai
3. Se ejecutará como servicio de Windows automáticamente
4. Puerto: `6379` (por defecto)

**Opción B: Redis en WSL2 (si tienes WSL)**
```bash
wsl
sudo apt update
sudo apt install redis-server -y
sudo service redis-server start
```

**Opción C: Compilar Redis desde código (avanzado)**
- Requiere Visual Studio y herramientas de compilación

### 3. Verificar Instalación

```powershell
# Verificar PostgreSQL
psql -U postgres -c "SELECT version();"

# Verificar Redis (si usas Memurai, usa el cliente de Memurai)
# O si usas WSL:
wsl redis-cli ping
```

### 4. Actualizar .env (si es necesario)

Si cambiaste la contraseña de PostgreSQL, actualiza el archivo `.env`:
```
DB_PASSWORD=tu_contraseña_aqui
```

### 5. Iniciar el Backend

```powershell
npm run dev
```

## 🔄 Alternativa: Actualizar Windows

Si quieres usar Docker Desktop en el futuro:

1. **Actualizar Windows 10 a la versión 22H2:**
   - Configuración > Actualización y seguridad > Windows Update
   - Busca actualizaciones y actualiza a la versión más reciente

2. **O actualizar a Windows 11** (si tu hardware es compatible)

## 📝 Notas

- PostgreSQL y Redis se ejecutarán como servicios de Windows
- Se iniciarán automáticamente al arrancar Windows
- No necesitas Docker para esto

