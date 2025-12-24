# Common Layer - Código Compartido

Capa común con DTOs, Enums, Guards, Utils y más. Evita duplicación de código.

## Estructura

```
common/
├── constants/     # Constantes compartidas
├── decorators/    # Decoradores personalizados
├── dto/          # DTOs compartidos
├── enums/        # Enums (OrderStatus, EscrowStatus, etc.)
├── filters/      # Exception filters
├── guards/       # Guards (Auth, RateLimit, Roles)
├── interceptors/ # Interceptors (Transform, Logging)
└── utils/        # Utilidades (Validation, Date, Format, Encryption)
```

## Enums

### OrderStatus
```typescript
import { OrderStatus } from '@/common';

OrderStatus.CREATED
OrderStatus.AWAITING_FUNDS
OrderStatus.ONCHAIN_LOCKED
OrderStatus.COMPLETED
OrderStatus.REFUNDED
OrderStatus.DISPUTED
```

### EscrowStatus
```typescript
import { EscrowStatus } from '@/common';

EscrowStatus.PENDING
EscrowStatus.LOCKED
EscrowStatus.RELEASED
EscrowStatus.REFUNDED
EscrowStatus.DISPUTED
```

### DisputeStatus
```typescript
import { DisputeStatus } from '@/common';

DisputeStatus.OPEN
DisputeStatus.IN_REVIEW
DisputeStatus.RESOLVED
DisputeStatus.CLOSED
DisputeStatus.ESCALATED
```

## DTOs Compartidos

### PaginationDto
```typescript
import { PaginationDto, PaginationResponseDto } from '@/common';

class MyController {
  @Get()
  async findAll(@Query() pagination: PaginationDto) {
    const { data, total } = await service.findAll(pagination.page, pagination.limit);
    return new PaginationResponseDto(data, total, pagination.page, pagination.limit);
  }
}
```

### ApiResponseDto
```typescript
import { ApiResponseDto } from '@/common';

// Respuesta exitosa
return ApiResponseDto.success(data, 'Operación exitosa');

// Respuesta de error
return ApiResponseDto.error('Error message', 'Descripción del error');
```

### ErrorResponseDto
Usado automáticamente por el HttpExceptionFilter.

## Guards

### JwtAuthGuard
```typescript
import { JwtAuthGuard } from '@/common';

@UseGuards(JwtAuthGuard)
@Get('protected')
async protectedRoute() {
  // Requiere autenticación
}
```

### RateLimitGuard
```typescript
import { RateLimitGuard, RateLimit } from '@/common';

@UseGuards(RateLimitGuard)
@RateLimit(10, 60) // 10 requests por 60 segundos
@Post('endpoint')
async limitedEndpoint() {
  // Rate limited
}
```

### RolesGuard
```typescript
import { RolesGuard } from '@/common';

@UseGuards(RolesGuard)
@Roles('admin')
@Get('admin')
async adminRoute() {
  // Requiere rol admin
}
```

## Decorators

### @CurrentUser()
```typescript
import { CurrentUser } from '@/common';

@Get('me')
@UseGuards(JwtAuthGuard)
async getProfile(@CurrentUser() user: User) {
  return user;
}
```

## Utils

### ValidationUtil
```typescript
import { ValidationUtil } from '@/common';

ValidationUtil.isValidEthereumAddress('0x...');
ValidationUtil.normalizeEthereumAddress('0x...');
ValidationUtil.isValidTransactionHash('0x...');
ValidationUtil.isValidNonce('0x...');
ValidationUtil.isValidUUID('uuid');
ValidationUtil.isInRange(50, 0, 100);
ValidationUtil.isNotEmpty('string');
```

### DateUtil
```typescript
import { DateUtil } from '@/common';

DateUtil.now();
DateUtil.addDays(date, 7);
DateUtil.addHours(date, 24);
DateUtil.addMinutes(date, 30);
DateUtil.isExpired(date);
DateUtil.isFuture(date);
DateUtil.diffInDays(date1, date2);
DateUtil.formatReadable(date);
```

### FormatUtil
```typescript
import { FormatUtil } from '@/common';

FormatUtil.formatDecimal(123.456, 2); // "123.46"
FormatUtil.shortenAddress('0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb'); // "0x742d...f0bEb"
FormatUtil.shortenHash('0x...'); // "0x12345678...abcdef"
FormatUtil.formatLargeNumber(1500000); // "1.50M"
FormatUtil.formatPercentage(85.5); // "85.50%"
FormatUtil.formatCurrency(1000, 'USD'); // "$1,000.00"
FormatUtil.capitalize('hello'); // "Hello"
FormatUtil.toCamelCase('snake_case'); // "snakeCase"
FormatUtil.toSnakeCase('camelCase'); // "camel_case"
```

### EncryptionUtil
```typescript
import { EncryptionUtil } from '@/common';

const encrypted = EncryptionUtil.encrypt('sensitive data', masterKey);
const decrypted = EncryptionUtil.decrypt(encrypted, masterKey);
```

## Interceptors

### TransformInterceptor
Transforma respuestas a formato estándar con statusCode y timestamp.

### LoggingInterceptor
Registra todas las peticiones HTTP con tiempo de respuesta.

## Filters

### HttpExceptionFilter
Filtro global para manejar excepciones HTTP con formato estándar.

## Constants

```typescript
import {
  DEFAULT_PAGE,
  DEFAULT_LIMIT,
  MAX_LIMIT,
  RATE_LIMIT_DEFAULT,
  NONCE_TTL_SECONDS,
  SESSION_TTL_SECONDS,
  REPUTATION_SCORE_MIN,
  REPUTATION_SCORE_MAX,
  DISPUTE_TIMERS,
  BLOCKCHAIN_SYNC,
} from '@/common';
```

## Uso

### Importación Centralizada

```typescript
// Importar todo desde common
import {
  OrderStatus,
  EscrowStatus,
  DisputeStatus,
  PaginationDto,
  ApiResponseDto,
  JwtAuthGuard,
  RateLimitGuard,
  RateLimit,
  CurrentUser,
  ValidationUtil,
  DateUtil,
  FormatUtil,
} from '@/common';
```

### Ejemplo Completo

```typescript
import {
  Controller,
  Get,
  Query,
  UseGuards,
} from '@nestjs/common';
import {
  PaginationDto,
  PaginationResponseDto,
  JwtAuthGuard,
  RateLimitGuard,
  RateLimit,
  CurrentUser,
  ValidationUtil,
} from '@/common';
import { User } from '@/database/entities/user.entity';

@Controller('example')
export class ExampleController {
  @Get()
  @UseGuards(RateLimitGuard)
  @RateLimit(20, 60)
  async findAll(@Query() pagination: PaginationDto) {
    // Usar paginación
    const { data, total } = await service.findAll(
      pagination.page || 1,
      pagination.limit || 20,
    );
    
    return new PaginationResponseDto(data, total, pagination.page, pagination.limit);
  }

  @Get('me')
  @UseGuards(JwtAuthGuard)
  async getProfile(@CurrentUser() user: User) {
    // Validar dirección
    if (!ValidationUtil.isValidEthereumAddress(user.walletAddress)) {
      throw new BadRequestException('Invalid address');
    }
    
    return ApiResponseDto.success(user, 'Perfil obtenido');
  }
}
```

## Principios

1. **DRY (Don't Repeat Yourself)**: Código compartido en common
2. **Single Source of Truth**: Enums y constantes centralizados
3. **Reutilización**: Guards, utils y DTOs reutilizables
4. **Consistencia**: Mismo formato en toda la aplicación
5. **Mantenibilidad**: Cambios en un solo lugar

## Mejores Prácticas

1. **Usar enums** en lugar de strings mágicos
2. **Usar DTOs compartidos** para respuestas consistentes
3. **Usar guards** para protección de rutas
4. **Usar utils** para lógica común
5. **Importar desde `@/common`** para evitar rutas relativas largas
