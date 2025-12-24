import { registerAs } from '@nestjs/config';

/**
 * Configuración de variables de entorno
 * 
 * Rol: Carga y validación de variables de entorno
 * 
 * Contiene:
 * - RPC URLs (blockchain)
 * - DB credentials (PostgreSQL)
 * - Redis config
 * - JWT config
 * - Rate limit config
 * - App config
 * 
 * Se conecta con:
 * - Todos los módulos que requieren config
 * - AppModule (carga todas las configs)
 * - ConfigService (inyectado globalmente)
 */

// ============================================
// APP CONFIG
// ============================================

export default registerAs('app', () => {
  const nodeEnv = process.env.NODE_ENV || 'development';
  const port = parseInt(process.env.PORT || '3000', 10);
  const corsOrigin = process.env.CORS_ORIGIN || '*';
  const domain = process.env.APP_DOMAIN || 'rsc.finance';

  // Validación
  if (isNaN(port) || port < 1 || port > 65535) {
    throw new Error('PORT must be a valid number between 1 and 65535');
  }

  return {
    nodeEnv,
    port,
    corsOrigin,
    domain,
  };
});

// ============================================
// DATABASE CONFIG (PostgreSQL)
// ============================================

export const databaseConfig = registerAs('database', () => {
  const host = process.env.DB_HOST || 'localhost';
  const port = parseInt(process.env.DB_PORT || '5432', 10);
  const username = process.env.DB_USERNAME || 'postgres';
  const password = process.env.DB_PASSWORD || 'postgres';
  const database = process.env.DB_DATABASE || 'rsc_db';

  // Validación
  if (isNaN(port) || port < 1 || port > 65535) {
    throw new Error('DB_PORT must be a valid number between 1 and 65535');
  }

  if (!database) {
    throw new Error('DB_DATABASE is required');
  }

  // Advertencias para producción
  if (process.env.NODE_ENV === 'production') {
    if (password === 'postgres' || password === '') {
      console.warn('⚠️  WARNING: Using default database password in production!');
    }
    if (host === 'localhost') {
      console.warn('⚠️  WARNING: Using localhost database in production!');
    }
  }

  return {
    host,
    port,
    username,
    password,
    database,
  };
});

// ============================================
// REDIS CONFIG
// ============================================

export const redisConfig = registerAs('redis', () => {
  const host = process.env.REDIS_HOST || 'localhost';
  const port = parseInt(process.env.REDIS_PORT || '6379', 10);
  const password = process.env.REDIS_PASSWORD || undefined;

  // Validación
  if (isNaN(port) || port < 1 || port > 65535) {
    throw new Error('REDIS_PORT must be a valid number between 1 and 65535');
  }

  // Advertencias para producción
  if (process.env.NODE_ENV === 'production') {
    if (host === 'localhost') {
      console.warn('⚠️  WARNING: Using localhost Redis in production!');
    }
  }

  return {
    host,
    port,
    password,
  };
});

// ============================================
// BLOCKCHAIN CONFIG
// ============================================

export const blockchainConfig = registerAs('blockchain', () => {
  // RPC URL - Puede ser vacío en desarrollo
  const rpcUrl = process.env.BLOCKCHAIN_RPC_URL || '';
  const network = process.env.BLOCKCHAIN_NETWORK || 'mainnet';
  
  // Private Key - SOLO para lectura, NUNCA debe tener fondos
  // ⚠️ REGLA FINAL: Este backend NUNCA debe mover fondos
  const privateKey = process.env.BLOCKCHAIN_PRIVATE_KEY || '';
  
  // Escrow Contract Address
  const escrowContractAddress = process.env.ESCROW_CONTRACT_ADDRESS || '';

  // Validación
  if (process.env.NODE_ENV === 'production') {
    if (!rpcUrl) {
      throw new Error('BLOCKCHAIN_RPC_URL is required in production');
    }
    if (!escrowContractAddress) {
      throw new Error('ESCROW_CONTRACT_ADDRESS is required in production');
    }
  }

  // Advertencias
  if (privateKey && privateKey !== '') {
    if (privateKey === 'your_private_key_here' || 
        privateKey.startsWith('0xyour_') ||
        privateKey.includes('your_private_key')) {
      console.warn('⚠️  WARNING: Using placeholder private key!');
    } else {
      console.warn('⚠️  WARNING: Private key configured. Ensure it has NO funds!');
      console.warn('⚠️  REGLA FINAL: Backend NUNCA debe mover fondos');
    }
  }

  return {
    rpcUrl,
    network,
    privateKey,
    escrowContractAddress,
  };
});

// ============================================
// JWT CONFIG
// ============================================

export const jwtConfig = registerAs('jwt', () => {
  const secret = process.env.JWT_SECRET || 'your-secret-key';
  const expiresIn = process.env.JWT_EXPIRES_IN || '24h';
  const refreshExpiresIn = process.env.JWT_REFRESH_EXPIRES_IN || '7d';

  // Validación
  if (process.env.NODE_ENV === 'production') {
    if (!secret || secret === 'your-secret-key') {
      throw new Error('JWT_SECRET must be set in production and must not be the default value');
    }
    if (secret.length < 32) {
      console.warn('⚠️  WARNING: JWT_SECRET should be at least 32 characters long');
    }
  }

  return {
    secret,
    expiresIn,
    refreshExpiresIn,
  };
});

// ============================================
// RATE LIMIT CONFIG
// ============================================

export const rateLimitConfig = registerAs('rateLimit', () => {
  const ttl = parseInt(process.env.RATE_LIMIT_TTL || '60', 10);
  const max = parseInt(process.env.RATE_LIMIT_MAX || '100', 10);

  // Validación
  if (isNaN(ttl) || ttl < 1) {
    throw new Error('RATE_LIMIT_TTL must be a positive number');
  }
  if (isNaN(max) || max < 1) {
    throw new Error('RATE_LIMIT_MAX must be a positive number');
  }

  return {
    ttl,
    max,
  };
});

// ============================================
// EXPORT ALL CONFIGS
// ============================================

/**
 * Todas las configuraciones exportadas
 * Se cargan en AppModule mediante ConfigModule.forRoot()
 */
export const allConfigs = [
  'app',
  'database',
  'redis',
  'blockchain',
  'jwt',
  'rateLimit',
] as const;
