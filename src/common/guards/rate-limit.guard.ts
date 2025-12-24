import {
  Injectable,
  CanActivate,
  ExecutionContext,
  HttpException,
  HttpStatus,
  Inject,
  SetMetadata,
} from '@nestjs/common';
import { Reflector } from '@nestjs/core';
import { Redis } from 'ioredis';

export const RATE_LIMIT_KEY = 'rateLimit';

export const RateLimit = (max: number, windowSeconds: number) => {
  return SetMetadata(RATE_LIMIT_KEY, { max, windowSeconds });
};

@Injectable()
export class RateLimitGuard implements CanActivate {
  constructor(
    private readonly reflector: Reflector,
    @Inject('REDIS_CLIENT')
    private readonly redis: Redis,
  ) {}

  async canActivate(context: ExecutionContext): Promise<boolean> {
    const request = context.switchToHttp().getRequest();
    const handler = context.getHandler();
    
    // Obtener configuración del decorador o usar defaults
    const rateLimitConfig = this.reflector.get<{ max: number; windowSeconds: number }>(
      RATE_LIMIT_KEY,
      handler,
    );

    if (!rateLimitConfig) {
      return true; // Sin rate limit configurado
    }

    const { max, windowSeconds } = rateLimitConfig;

    // Identificar cliente (IP o userId)
    const identifier = this.getIdentifier(request);
    const key = `rate-limit:${identifier}:${handler.name}`;

    try {
      const current = await this.redis.incr(key);
      
      if (current === 1) {
        await this.redis.expire(key, windowSeconds);
      }

      if (current > max) {
        const ttl = await this.redis.ttl(key);
        throw new HttpException(
          {
            statusCode: HttpStatus.TOO_MANY_REQUESTS,
            message: `Demasiadas solicitudes. Intenta de nuevo en ${ttl} segundos.`,
            error: 'Too Many Requests',
          },
          HttpStatus.TOO_MANY_REQUESTS,
        );
      }

      return true;
    } catch (error) {
      if (error instanceof HttpException) {
        throw error;
      }
      // Si Redis falla, permitir la solicitud (no crítico)
      return true;
    }
  }

  private getIdentifier(request: any): string {
    // Priorizar userId si está autenticado
    if (request.user?.id) {
      return `user:${request.user.id}`;
    }
    
    // Usar IP como fallback
    const ip = request.ip || request.connection?.remoteAddress || 'unknown';
    return `ip:${ip}`;
  }
}
