import { Controller, Get } from '@nestjs/common';
import { HealthService, HealthCheckResult } from './health.service';

/**
 * Health Check Controller
 * 
 * Endpoints:
 * - GET /health/live - Liveness probe (Kubernetes)
 * - GET /health/ready - Readiness probe (Kubernetes)
 * - GET /health - Health check completo
 */
@Controller('health')
export class HealthController {
  constructor(private readonly healthService: HealthService) {}

  /**
   * Liveness probe
   * 
   * Usado por Kubernetes para verificar que el pod está vivo
   */
  @Get('live')
  async liveness() {
    return this.healthService.checkLiveness();
  }

  /**
   * Readiness probe
   * 
   * Usado por Kubernetes para verificar que el pod está listo
   * para recibir tráfico
   */
  @Get('ready')
  async readiness() {
    return this.healthService.checkReadiness();
  }

  /**
   * Health check completo
   * 
   * Proporciona información detallada sobre el estado de la aplicación
   * y todas sus dependencias
   */
  @Get()
  async health(): Promise<{
    status: 'healthy' | 'degraded' | 'unhealthy';
    timestamp: string;
    uptime: number;
    checks: {
      database: HealthCheckResult;
      redis: HealthCheckResult;
      blockchain: HealthCheckResult;
    };
  }> {
    return this.healthService.checkHealth();
  }
}

