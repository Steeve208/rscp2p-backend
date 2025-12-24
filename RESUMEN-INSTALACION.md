# Resumen de Instalación - Docker Desktop

## ✅ Lo que se ha hecho:

1. **Script de instalación creado**: `install-docker.ps1`
2. **Ventana de PowerShell con privilegios elevados abierta** para instalar Docker Desktop

## 📋 Próximos Pasos:

### Si la instalación se completó:

1. **REINICIA tu computadora** (requerido por Docker Desktop)
2. **Inicia Docker Desktop** desde el menú de inicio
3. **Espera** a que Docker Desktop esté completamente iniciado (ícono verde en la bandeja del sistema)
4. **Ejecuta los servicios**:
   ```powershell
   .\start-services.ps1
   ```
5. **Inicia el backend**:
   ```powershell
   npm run dev
   ```

### Si la instalación no se completó:

**Opción 1: Ejecutar script manualmente**
```powershell
# Abre PowerShell como Administrador
.\install-docker.ps1
```

**Opción 2: Instalar con winget manualmente**
```powershell
# En PowerShell como Administrador
winget install --id Docker.DockerDesktop --accept-package-agreements --accept-source-agreements
```

**Opción 3: Descargar e instalar manualmente**
- Visita: https://www.docker.com/products/docker-desktop/
- Descarga e instala Docker Desktop
- Reinicia Windows

## 🔍 Verificar Instalación:

```powershell
# Verificar versión de Docker
docker --version

# Verificar que Docker está corriendo
docker ps
```

## 🚀 Una vez Docker esté instalado y corriendo:

```powershell
# Iniciar PostgreSQL y Redis
.\start-services.ps1

# O manualmente
docker-compose up -d

# Iniciar el backend
npm run dev
```

## 📝 Notas:

- Docker Desktop requiere reiniciar Windows después de la instalación
- Docker Desktop debe estar corriendo (ícono verde) antes de usar docker-compose
- Los servicios PostgreSQL y Redis se iniciarán automáticamente con `start-services.ps1`

