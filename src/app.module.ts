import { Module } from '@nestjs/common';
import { ConfigModule, ConfigService } from '@nestjs/config';
import { TypeOrmModule } from '@nestjs/typeorm';
import { getDatabaseConfig } from './config/database';
import { createRedisProvider } from './config/redis';
import appConfig, {
  databaseConfig,
  redisConfig,
  blockchainConfig,
  jwtConfig,
  rateLimitConfig,
} from './config/env';

// ============================================
// MÓDULOS DE APLICACIÓN
// ============================================
import { AuthModule } from './modules/auth/auth.module';
import { UsersModule } from './modules/users/users.module';
import { OrdersModule } from './modules/orders/orders.module';
import { EscrowModule } from './modules/escrow/escrow.module';
import { ReputationModule } from './modules/reputation/reputation.module';
import { DisputesModule } from './modules/disputes/disputes.module';
import { LaunchpadModule } from './modules/launchpad/launchpad.module';

// ============================================
// MÓDULOS DE INFRAESTRUCTURA
// ============================================
import { DatabaseModule } from './database/database.module';
import { AuditModule } from './common/audit/audit.module';
import { HealthModule } from './common/health/health.module';

/**
 * Módulo raíz de la aplicación
 * 
 * Responsabilidades:
 * - Importación de todos los módulos del sistema
 * - Configuración global (Database, Redis, Config)
 * - Orquestación de módulos de aplicación e infraestructura
 * 
 * Estructura:
 * 1. Configuración (ConfigModule, TypeORM, Schedule)
 * 2. Infraestructura (Database, WebSocket, Jobs)
 * 3. Módulos de aplicación (Auth, Users, Orders, etc.)
 * 
 * Se conecta con:
 * - Todos los módulos del sistema
 * - Configuración global
 * - Servicios de infraestructura
 */
@Module({
  imports: [
    // ============================================
    // CONFIGURACIÓN GLOBAL
    // ============================================
    
    // ConfigModule - Configuración global desde .env
    ConfigModule.forRoot({
      isGlobal: true,
      load: [
        appConfig,
        databaseConfig,
        redisConfig,
        blockchainConfig,
        jwtConfig,
        rateLimitConfig,
      ],
      envFilePath: '.env',
    }),

    // TypeORM - Conexión a PostgreSQL
    TypeOrmModule.forRootAsync({
      imports: [ConfigModule],
      useFactory: (configService: ConfigService) => getDatabaseConfig(configService),
      inject: [ConfigService],
    }),

    // ============================================
    // INFRAESTRUCTURA
    // ============================================
    
    // DatabaseModule - Servicios Redis (locks, sessions, rate limit)
    DatabaseModule,

    // AuditModule - Sistema de auditoría de seguridad
    AuditModule,

    // HealthModule - Health check (/api/health, /api/health/ready, /api/health/live)
    HealthModule,

    // ============================================
    // MÓDULOS DE APLICACIÓN
    // ============================================
    
    // AuthModule - Autenticación con wallet (sin emails/passwords)
    AuthModule,

    // UsersModule - Usuarios pseudónimos ligados a wallets
    UsersModule,

    // OrdersModule - Núcleo del mercado P2P (crear, aceptar, cancelar)
    OrdersModule,

    // EscrowModule - Mapeo order_id ↔ escrow_id y validación
    EscrowModule,

    // ReputationModule - Sistema de confianza off-chain
    ReputationModule,

    // DisputesModule - Gestión de conflictos humanos
    DisputesModule,

    // LaunchpadModule - Launchpad API + WS
    LaunchpadModule,
  ],
  providers: [
    // Redis Provider - Cliente Redis global
    {
      ...createRedisProvider(new ConfigService()),
      inject: [ConfigService],
    },
  ],
})
export class AppModule {}

