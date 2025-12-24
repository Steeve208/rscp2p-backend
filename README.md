# RSC Backend - Sistema P2P de Finanzas

Backend desarrollado con NestJS para la plataforma RSC Finance, un sistema P2P de intercambio con integración blockchain.

## 📋 Tabla de Contenidos

- [Arquitectura](#arquitectura)
- [Requisitos](#requisitos)
- [Instalación](#instalación)
- [Configuración](#configuración)
- [Estructura del Proyecto](#estructura-del-proyecto)
- [Scripts Disponibles](#scripts-disponibles)
- [Módulos](#módulos)
- [Base de Datos](#base-de-datos)
- [Blockchain](#blockchain)
- [WebSocket](#websocket)
- [Desarrollo](#desarrollo)

## 🏗️ Arquitectura

El proyecto sigue una arquitectura modular basada en NestJS:

```
backend/
├── src/
│   ├── app.module.ts          # Módulo principal
│   ├── main.ts                # Punto de entrada
│   │
│   ├── config/                # Configuraciones
│   │   ├── env.ts             # Variables de entorno
│   │   ├── database.ts        # Configuración de base de datos
│   │   ├── redis.ts           # Configuración de Redis
│   │   └── blockchain.ts      # Configuración de blockchain
│   │
│   ├── modules/               # Módulos de la aplicación
│   │   ├── auth/              # Autenticación
│   │   ├── users/             # Gestión de usuarios
│   │   ├── orders/            # Órdenes P2P
│   │   ├── escrow/            # Gestión de escrow
│   │   ├── blockchain/        # Integración blockchain
│   │   ├── reputation/        # Sistema de reputación
│   │   ├── disputes/          # Gestión de disputas
│   │   └── notifications/     # Notificaciones
│   │
│   ├── common/                # Código compartido
│   │   ├── dto/               # Data Transfer Objects
│   │   ├── enums/             # Enumeraciones
│   │   ├── guards/            # Guards de seguridad
│   │   ├── interceptors/      # Interceptores
│   │   ├── filters/           # Filtros de excepciones
│   │   └── utils/             # Utilidades
│   │
│   ├── database/              # Base de datos
│   │   ├── entities/          # Entidades TypeORM
│   │   ├── migrations/        # Migraciones
│   │   └── seeds/             # Seeds de datos
│   │
│   ├── jobs/                  # Jobs programados
│   │   ├── blockchain-sync.job.ts
│   │   └── cleanup.job.ts
│   │
│   └── websocket/             # WebSocket
│       └── market.gateway.ts
│
├── test/                      # Tests
├── .env.example               # Ejemplo de variables de entorno
├── package.json
└── README.md
```

## 📦 Requisitos

- Node.js >= 18.x
- PostgreSQL >= 14.x
- Redis >= 6.x
- npm o yarn

## 🚀 Instalación

1. Clonar el repositorio:
```bash
git clone <repository-url>
cd p2p-backend
```

2. Instalar dependencias:
```bash
npm install
```

3. Configurar variables de entorno:
```bash
cp .env.example .env
# Editar .env con tus configuraciones
```

## ⚙️ Configuración

### Variables de Entorno

Crear un archivo `.env` en la raíz del proyecto con las siguientes variables:

```env
# Server
NODE_ENV=development
PORT=3000

# Database
DB_HOST=localhost
DB_PORT=5432
DB_USERNAME=postgres
DB_PASSWORD=postgres
DB_DATABASE=rsc_db

# Redis
REDIS_HOST=localhost
REDIS_PORT=6379
REDIS_PASSWORD=

# Blockchain
BLOCKCHAIN_RPC_URL=https://mainnet.infura.io/v3/YOUR_PROJECT_ID
BLOCKCHAIN_NETWORK=mainnet
BLOCKCHAIN_PRIVATE_KEY=your_private_key_here
ESCROW_CONTRACT_ADDRESS=

# JWT
JWT_SECRET=your_jwt_secret_here
JWT_EXPIRES_IN=24h

# Rate Limiting
RATE_LIMIT_TTL=60
RATE_LIMIT_MAX=100

# CORS
CORS_ORIGIN=http://localhost:3000
```

### Base de Datos

1. Crear la base de datos PostgreSQL:
```bash
createdb rsc_db
```

2. Ejecutar migraciones:
```bash
npm run migration:run
```

## 📜 Scripts Disponibles

```bash
# Desarrollo
npm run dev              # Inicia en modo desarrollo con hot-reload

# Producción
npm run build            # Compila el proyecto
npm run start:prod       # Inicia en modo producción

# Base de datos
npm run migration:generate  # Genera una nueva migración
npm run migration:run       # Ejecuta migraciones pendientes
npm run migration:revert    # Revierte la última migración

# Calidad de código
npm run lint             # Ejecuta ESLint
npm run format            # Formatea código con Prettier

# Testing
npm run test             # Ejecuta tests unitarios
npm run test:watch        # Ejecuta tests en modo watch
npm run test:cov          # Ejecuta tests con cobertura
npm run test:e2e          # Ejecuta tests end-to-end
```

## 🔧 Módulos

### Auth Module
Maneja autenticación y autorización de usuarios.

### Users Module
Gestión de usuarios del sistema.

### Orders Module
Gestión de órdenes P2P (crear, actualizar, cancelar).

### Escrow Module
Gestión de fondos en custodia (escrow) con integración blockchain.

### Blockchain Module
Integración con blockchain (transacciones, balances, estado).

### Reputation Module
Sistema de reputación y calificaciones de usuarios.

### Disputes Module
Gestión de disputas entre usuarios.

### Notifications Module
Sistema de notificaciones en tiempo real.

## 🗄️ Base de Datos

El proyecto utiliza TypeORM con PostgreSQL. Las entidades principales son:

- **User**: Usuarios del sistema
- **Order**: Órdenes P2P
- **Escrow**: Fondos en custodia
- **Dispute**: Disputas

### Migraciones

Las migraciones se encuentran en `src/database/migrations/`. Para crear una nueva migración:

```bash
npm run migration:generate -- -n NombreMigracion
```

## ⛓️ Blockchain

El proyecto utiliza **ethers.js** para interactuar con la blockchain. La configuración se encuentra en `src/config/blockchain.ts`.

### Funcionalidades

- Conexión a red blockchain (Ethereum)
- Envío de transacciones
- Consulta de balances
- Interacción con contratos inteligentes (escrow)

## 🔌 WebSocket

El gateway de WebSocket (`market.gateway.ts`) permite comunicación en tiempo real para:

- Actualizaciones de órdenes
- Actualizaciones de mercado
- Actualizaciones de precios

### Uso del cliente

```javascript
const socket = io('http://localhost:3000/market');

// Suscribirse a actualizaciones
socket.emit('subscribe', { channel: 'order:123' });

// Escuchar actualizaciones
socket.on('order:update', (data) => {
  console.log('Orden actualizada:', data);
});
```

## 🛠️ Desarrollo

### Estructura de un Módulo

Cada módulo sigue esta estructura:

```
module-name/
├── module-name.module.ts    # Definición del módulo
├── module-name.controller.ts # Controlador REST
└── module-name.service.ts    # Lógica de negocio
```

### Guards

Los guards disponibles son:

- `JwtAuthGuard`: Verifica tokens JWT
- `RolesGuard`: Verifica roles de usuario

### Interceptores

- `TransformInterceptor`: Transforma respuestas
- `LoggingInterceptor`: Registra peticiones

### Filtros

- `HttpExceptionFilter`: Maneja excepciones HTTP

## 📝 Notas Importantes

### Dependencias

⚠️ **IMPORTANTE**: Todas las dependencias se instalan a nivel del proyecto backend, NO por carpeta. El `package.json` gobierna todas las dependencias del proyecto.

### ORM

El proyecto utiliza **TypeORM**. No se debe mezclar con otros ORMs como Prisma.

### Blockchain SDK

El proyecto utiliza **ethers.js**. No se debe mezclar con otros SDKs como web3.js o viem.

## 🤝 Contribución

1. Crear una rama para la feature
2. Realizar los cambios
3. Ejecutar tests y linter
4. Crear un Pull Request

## 📄 Licencia

UNLICENSED

