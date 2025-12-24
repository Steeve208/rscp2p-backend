# Módulo de Usuarios

Sistema de usuarios pseudónimos ligados a wallets, sin información personal.

## Características

- ✅ Usuarios pseudónimos (solo wallet address)
- ✅ Sistema de reputation score
- ✅ Nunca expone información personal
- ✅ Endpoints públicos y protegidos
- ✅ Ranking de usuarios
- ✅ Búsqueda por wallet address

## Estructura de Datos

### Entidad User

```typescript
{
  id: string;                    // UUID
  walletAddress: string;         // Dirección Ethereum (única)
  reputationScore: number;        // Score de reputación (-100 a 100)
  createdAt: Date;                // Fecha de creación
  isActive: boolean;              // Estado activo/inactivo
  lastLoginAt: Date;             // Último login (interno)
  loginCount: number;             // Contador de logins (interno)
}
```

### Datos Públicos Expuestos

**NUNCA se expone información personal.** Solo se expone:

- `id`: Identificador único
- `wallet_address`: Dirección de la wallet
- `reputation_score`: Score de reputación
- `created_at`: Fecha de creación

## Endpoints

### Públicos (sin autenticación)

#### Listar Usuarios
```http
GET /api/users?page=1&limit=20&search=0x...
```

**Respuesta:**
```json
{
  "data": [
    {
      "id": "uuid",
      "wallet_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
      "reputation_score": 85.5,
      "created_at": "2024-01-01T00:00:00.000Z"
    }
  ],
  "total": 100,
  "page": 1,
  "limit": 20,
  "totalPages": 5
}
```

#### Obtener Usuario por ID
```http
GET /api/users/:id
```

#### Obtener Usuario por Wallet
```http
GET /api/users/wallet/:address
```

#### Ranking de Usuarios
```http
GET /api/users/ranking?limit=100
```

**Respuesta:**
```json
[
  {
    "id": "uuid",
    "wallet_address": "0x...",
    "reputation_score": 95.0,
    "created_at": "2024-01-01T00:00:00.000Z"
  }
]
```

#### Estadísticas de Usuario
```http
GET /api/users/stats/:address
```

**Respuesta:**
```json
{
  "walletAddress": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
  "reputationScore": 85.5,
  "createdAt": "2024-01-01T00:00:00.000Z",
  "rank": 42
}
```

### Protegidos (requieren autenticación)

#### Perfil Completo del Usuario Autenticado
```http
GET /api/users/me/profile
Authorization: Bearer <token>
```

**Respuesta:**
```json
{
  "id": "uuid",
  "walletAddress": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
  "reputationScore": 85.5,
  "createdAt": "2024-01-01T00:00:00.000Z",
  "isActive": true,
  "lastLoginAt": "2024-01-15T10:30:00.000Z",
  "loginCount": 42
}
```

## Reputation Score

El reputation score es un valor entre **-100 y 100** que refleja la reputación del usuario en la plataforma.

### Actualización del Score

El score se actualiza automáticamente mediante el método `updateReputationScore()`:

```typescript
// Incrementar score
await usersService.updateReputationScore(userId, 5);

// Decrementar score
await usersService.updateReputationScore(userId, -10);

// Establecer score directamente
await usersService.setReputationScore(userId, 85);
```

### Factores que Afectan el Score

- ✅ Completar órdenes exitosamente: +5
- ❌ Cancelar órdenes: -10
- ✅ Resolver disputas a favor: +10
- ❌ Abrir disputas infundadas: -15
- ✅ Calificaciones positivas: +2
- ❌ Calificaciones negativas: -5

## Privacidad y Seguridad

### Principios

1. **Nunca información personal**: No se almacena ni expone:
   - Nombres
   - Emails
   - Direcciones físicas
   - Números de teléfono
   - Cualquier dato identificable

2. **Solo wallet address**: El único identificador es la dirección de la wallet Ethereum.

3. **Datos públicos limitados**: Solo se expone:
   - Wallet address
   - Reputation score
   - Fecha de creación

4. **Datos internos protegidos**: Información como `lastLoginAt` y `loginCount` solo es accesible por el propio usuario autenticado.

## Uso en Otros Módulos

### Actualizar Reputation desde Orders/Disputes

```typescript
import { UsersService } from '../users/users.service';

// En el servicio de orders
constructor(
  private readonly usersService: UsersService,
) {}

async completeOrder(orderId: string) {
  // ... lógica de completar orden
  
  // Actualizar reputation del vendedor
  await this.usersService.updateReputationScore(
    order.sellerId,
    5, // +5 por completar orden exitosamente
  );
}
```

## Ejemplo de Uso

```typescript
// Obtener usuario por wallet
const user = await fetch('/api/users/wallet/0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb')
  .then(r => r.json());

// Ver ranking
const ranking = await fetch('/api/users/ranking?limit=10')
  .then(r => r.json());

// Ver estadísticas
const stats = await fetch('/api/users/stats/0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb')
  .then(r => r.json());

// Perfil propio (requiere autenticación)
const profile = await fetch('/api/users/me/profile', {
  headers: {
    'Authorization': `Bearer ${accessToken}`,
  },
}).then(r => r.json());
```
