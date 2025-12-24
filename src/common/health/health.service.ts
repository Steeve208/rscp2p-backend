import { Injectable, Logger, Inject } from '@nestjs/common';
import { InjectConnection } from '@nestjs/typeorm';
import { Connection } from 'typeorm';
import { Redis } from 'ioredis';
import { ConfigService } from '@nestjs/config';

/**
 * Health Check Service
 * 
 * Proporciona health checks avanzados para:
 * - Liveness: ¿Está la aplicación viva?
 * - Readiness: ¿Está lista para recibir tráfico?
 * - Dependencies: Estado de dependencias (DB, Redis, Blockchain)
 */
@Injectable()
export class HealthService {
  private readonly logger = new Logger(HealthService.name);

  constructor(
    @InjectConnection()
    private readonly dbConnection: Connection,
    @Inject('REDIS_CLIENT')
    private readonly redis: Redis,
    private readonly configService: ConfigService,
  ) {}

  /**
   * Health check de liveness
   * 
   * Verifica que la aplicación está viva y funcionando
   */
  async checkLiveness(): Promise<{
    status: 'ok' | 'error';
    timestamp: string;
    uptime: number;
  }> {
    try {
      return {
        status: 'ok',
        timestamp: new Date().toISOString(),
        uptime: process.uptime(),
      };
    } catch (error) {
      this.logger.error(`Liveness check failed: ${error.message}`);
      return {
        status: 'error',
        timestamp: new Date().toISOString(),
        uptime: process.uptime(),
      };
    }
  }

  /**
   * Health check de readiness
   * 
   * Verifica que la aplicación está lista para recibir tráfico
   * (dependencias críticas funcionando)
   */
  async checkReadiness(): Promise<{
    status: 'ready' | 'not_ready';
    timestamp: string;
    checks: {
      database: 'ok' | 'error';
      redis: 'ok' | 'error';
    };
  }> {
    const checks = {
      database: 'ok' as 'ok' | 'error',
      redis: 'ok' as 'ok' | 'error',
    };

    // Verificar base de datos
    try {
      await this.dbConnection.query('SELECT 1');
    } catch (error) {
      this.logger.error(`Database health check failed: ${error.message}`);
      checks.database = 'error';
    }

    // Verificar Redis
    try {
      await this.redis.ping();
    } catch (error) {
      this.logger.error(`Redis health check failed: ${error.message}`);
      checks.redis = 'error';
    }

    const isReady = checks.database === 'ok' && checks.redis === 'ok';

    return {
      status: isReady ? 'ready' : 'not_ready',
      timestamp: new Date().toISOString(),
      checks,
    };
  }

  /**
   * Health check completo con todas las dependencias
   */
  async checkHealth(): Promise<{
    status: 'healthy' | 'degraded' | 'unhealthy';
    timestamp: string;
    uptime: number;
    checks: {
      database: HealthCheckResult;
      redis: HealthCheckResult;
      blockchain: HealthCheckResult;
    };
  }> {
    const checks = {
      database: await this.checkDatabase(),
      redis: await this.checkRedis(),
      blockchain: await this.checkBlockchain(),
    };

    // Determinar estado general
    const allHealthy = Object.values(checks).every((c) => c.status === 'ok');
    const anyCriticalFailed =
      checks.database.status === 'error' || checks.redis.status === 'error';

    let status: 'healthy' | 'degraded' | 'unhealthy';
    if (allHealthy) {
      status = 'healthy';
    } else if (anyCriticalFailed) {
      status = 'unhealthy';
    } else {
      status = 'degraded'; // Blockchain puede fallar sin ser crítico
    }

    return {
      status,
      timestamp: new Date().toISOString(),
      uptime: process.uptime(),
      checks,
    };
  }

  /**
   * Verifica el estado de la base de datos
   */
  private async checkDatabase(): Promise<HealthCheckResult> {
    try {
      const start = Date.now();
      await this.dbConnection.query('SELECT 1');
      const latency = Date.now() - start;

      // Obtener información de la conexión
      const isConnected = this.dbConnection.isConnected;
      const poolSize = (this.dbConnection.driver as any)?.pool?.totalCount || 0;

      return {
        status: 'ok',
        latency,
        details: {
          connected: isConnected,
          poolSize,
        },
      };
    } catch (error) {
      return {
        status: 'error',
        error: error.message,
      };
    }
  }

  /**
   * Verifica el estado de Redis
   */
  private async checkRedis(): Promise<HealthCheckResult> {
    try {
      const start = Date.now();
      await this.redis.ping();
      const latency = Date.now() - start;

      const info = await this.redis.info('server');
      const connectedClients = info.match(/connected_clients:(\d+)/)?.[1] || '0';

      return {
        status: 'ok',
        latency,
        details: {
          connectedClients: parseInt(connectedClients, 10),
        },
      };
    } catch (error) {
      return {
        status: 'error',
        error: error.message,
      };
    }
  }

  /**
   * Verifica el estado de blockchain (no crítico)
   */
  private async checkBlockchain(): Promise<HealthCheckResult> {
    try {
      const blockchainConfig = this.configService.get('blockchain');
      const rpcUrl = blockchainConfig?.rpcUrl;

      if (!rpcUrl || rpcUrl === '') {
        return {
          status: 'ok',
          details: {
            message: 'Blockchain not configured (development mode)',
          },
        };
      }

      // Intentar hacer una llamada simple al RPC
      // Nota: Esto requiere acceso al provider, que está en BlockchainModule
      // Por ahora, solo verificamos que la configuración existe
      return {
        status: 'ok',
        details: {
          rpcUrl: rpcUrl.substring(0, 20) + '...', // Ocultar URL completa
          configured: true,
        },
      };
    } catch (error) {
      return {
        status: 'error',
        error: error.message,
      };
    }
  }
}

/**
 * Resultado de un health check
 */
export interface HealthCheckResult {
  status: 'ok' | 'error';
  latency?: number;
  error?: string;
  details?: Record<string, any>;
}

