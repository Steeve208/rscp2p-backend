# Instalación de Docker Desktop para Windows

## Pasos para Instalar Docker Desktop

1. **Descargar Docker Desktop**
   - Visita: https://www.docker.com/products/docker-desktop/
   - Descarga la versión para Windows
   - Ejecuta el instalador

2. **Configuración Inicial**
   - Acepta los términos de licencia
   - Marca la opción "Use WSL 2 instead of Hyper-V" (recomendado)
   - Completa la instalación

3. **Reiniciar el Sistema**
   - Docker Desktop requiere reiniciar Windows

4. **Iniciar Docker Desktop**
   - Busca "Docker Desktop" en el menú de inicio
   - Inicia la aplicación
   - Espera a que el ícono de Docker aparezca en la bandeja del sistema (verde = corriendo)

5. **Verificar Instalación**
   ```powershell
   docker --version
   docker ps
   ```

## Una vez Docker esté instalado

Ejecuta el script para iniciar PostgreSQL y Redis:

```powershell
.\start-services.ps1
```

O manualmente:

```powershell
docker-compose up -d
```

## Comandos Útiles

```powershell
# Ver estado de los servicios
docker-compose ps

# Ver logs
docker-compose logs -f

# Detener servicios
docker-compose down

# Detener y eliminar volúmenes (borra datos)
docker-compose down -v
```

