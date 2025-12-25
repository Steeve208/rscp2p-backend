# Cómo Subir los Scripts al Servidor

Los scripts de configuración están en tu máquina local y necesitas subirlos al servidor de Digital Ocean.

## Opción 1: Usar el Script PowerShell (Recomendado)

Si estás en Windows:

```powershell
.\subir-scripts-db.ps1
```

El script te pedirá:
- IP del servidor
- Usuario (por defecto: root)
- Y subirá automáticamente todos los scripts necesarios

## Opción 2: Usar SCP Manualmente

### Desde Windows (PowerShell o CMD)

```powershell
# Reemplaza 123.45.67.89 con la IP de tu servidor
scp setup-postgresql.sh root@123.45.67.89:/var/www/p2prsc-backend/
scp verificar-db.sh root@123.45.67.89:/var/www/p2prsc-backend/
scp verificar-redis.sh root@123.45.67.89:/var/www/p2prsc-backend/
```

### Desde Linux/Mac

```bash
# Reemplaza 123.45.67.89 con la IP de tu servidor
scp setup-postgresql.sh root@123.45.67.89:/var/www/p2prsc-backend/
scp verificar-db.sh root@123.45.67.89:/var/www/p2prsc-backend/
scp verificar-redis.sh root@123.45.67.89:/var/www/p2prsc-backend/
```

## Opción 3: Crear los Scripts Directamente en el Servidor

Si prefieres, puedes copiar y pegar el contenido directamente en el servidor:

```bash
# En el servidor
cd /var/www/p2prsc-backend
nano setup-postgresql.sh
# Pega el contenido, guarda con Ctrl+X, luego Y, luego Enter
```

## Después de Subir los Archivos

Una vez que los archivos estén en el servidor:

```bash
cd /var/www/p2prsc-backend
chmod +x setup-postgresql.sh verificar-db.sh verificar-redis.sh
./setup-postgresql.sh
```

## Verificar que los Archivos Están en el Servidor

```bash
ls -la /var/www/p2prsc-backend/*.sh
```

Deberías ver:
- setup-postgresql.sh
- verificar-db.sh
- verificar-redis.sh

