import { Module } from '@nestjs/common';
import { ConfigModule, ConfigService } from '@nestjs/config';
import { TypeOrmModule } from '@nestjs/typeorm';
import { ScheduleModule } from '@nestjs/schedule';
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
import { BlockchainModule } from './modules/blockchain/blockchain.module';
import { ReputationModule } from './modules/reputation/reputation.module';
import { DisputesModule } from './modules/disputes/disputes.module';
import { NotificationsModule } from './modules/notifications/notifications.module';

// ============================================
// MÓDULOS DE INFRAESTRUCTURA
// ============================================
import { DatabaseModule } from './database/database.module';
import { JobsModule } from './jobs/jobs.module';
import { WebSocketModule } from './websocket/websocket.module';
import { HealthModule } from './common/health/health.module';
import { CircuitBreakerModule } from './common/circuit-breaker/circuit-breaker.module';
import { AuditModule } from './common/audit/audit.module';

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

    // ScheduleModule - Jobs y tareas programadas
    ScheduleModule.forRoot(),

    // ============================================
    // INFRAESTRUCTURA
    // ============================================
    
    // DatabaseModule - Servicios Redis (locks, sessions, rate limit)
    DatabaseModule,

    // WebSocketModule - WebSocket Gateway para notificaciones
    WebSocketModule,

    // JobsModule - Jobs críticos (blockchain sync, cleanup, consistency)
    JobsModule,

    // HealthModule - Health checks avanzados (liveness, readiness)
    HealthModule,

    // CircuitBreakerModule - Circuit breakers para servicios externos
    CircuitBreakerModule,

    // AuditModule - Sistema de auditoría de seguridad
    AuditModule,

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

    // BlockchainModule - Escucha eventos y reconcilia estados
    BlockchainModule,

    // ReputationModule - Sistema de confianza off-chain
    ReputationModule,

    // DisputesModule - Gestión de conflictos humanos
    DisputesModule,

    // NotificationsModule - Notificaciones WebSocket
    NotificationsModule,
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

