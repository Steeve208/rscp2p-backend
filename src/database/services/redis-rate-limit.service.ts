import { Injectable, Inject, Logger } from '@nestjs/common';
import { Redis } from 'ioredis';

@Injectable()
export class RedisRateLimitService {
  private readonly logger = new Logger(RedisRateLimitService.name);
  private readonly rateLimitPrefix = 'ratelimit:';

  constructor(
    @Inject('REDIS_CLIENT')
    private readonly redis: Redis,
  ) {}

  /**
   * Verifica rate limit para un identificador
   * Retorna { allowed: boolean, remaining: number, resetAt: Date }
   */
  async checkRateLimit(
    identifier: string,
    maxRequests: number,
    windowSeconds: number,
  ): Promise<{
    allowed: boolean;
    remaining: number;
    resetAt: Date;
  }> {
    const key = `${this.rateLimitPrefix}${identifier}`;
    
    try {
      const current = await this.redis.incr(key);
      
      if (current === 1) {
        await this.redis.expire(key, windowSeconds);
      }

      const ttl = await this.redis.ttl(key);
      const resetAt = new Date(Date.now() + ttl * 1000);
      const remaining = Math.max(0, maxRequests - current);

      return {
        allowed: current <= maxRequests,
        remaining,
        resetAt,
      };
    } catch (error) {
      this.logger.error(`Error checking rate limit for ${identifier}: ${error.message}`);
      // Si Redis falla, permitir la solicitud (no crítico)
      return {
        allowed: true,
        remaining: maxRequests,
        resetAt: new Date(),
      };
    }
  }

  /**
   * Resetea el rate limit para un identificador
   */
  async resetRateLimit(identifier: string): Promise<void> {
    const key = `${this.rateLimitPrefix}${identifier}`;
    
    try {
      await this.redis.del(key);
    } catch (error) {
      this.logger.error(`Error resetting rate limit for ${identifier}: ${error.message}`);
    }
  }

  /**
   * Obtiene información del rate limit sin incrementar
   */
  async getRateLimitInfo(
    identifier: string,
    maxRequests: number,
  ): Promise<{
    current: number;
    remaining: number;
    resetAt: Date;
  }> {
    const key = `${this.rateLimitPrefix}${identifier}`;
    
    try {
      const current = parseInt((await this.redis.get(key)) || '0', 10);
      const ttl = await this.redis.ttl(key);
      const resetAt = ttl > 0 ? new Date(Date.now() + ttl * 1000) : new Date();

      return {
        current,
        remaining: Math.max(0, maxRequests - current),
        resetAt,
      };
    } catch (error) {
      this.logger.error(`Error getting rate limit info for ${identifier}: ${error.message}`);
      return {
        current: 0,
        remaining: maxRequests,
        resetAt: new Date(),
      };
    }
  }
}
