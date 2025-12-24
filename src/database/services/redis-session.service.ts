import { Injectable, Inject, Logger } from '@nestjs/common';
import { Redis } from 'ioredis';

@Injectable()
export class RedisSessionService {
  private readonly logger = new Logger(RedisSessionService.name);
  private readonly sessionPrefix = 'session:';
  private readonly defaultTtl = 86400; // 24 horas

  constructor(
    @Inject('REDIS_CLIENT')
    private readonly redis: Redis,
  ) {}

  /**
   * Crea o actualiza una sesión
   */
  async setSession(
    sessionId: string,
    data: any,
    ttl: number = this.defaultTtl,
  ): Promise<void> {
    const key = `${this.sessionPrefix}${sessionId}`;
    
    try {
      await this.redis.setex(key, ttl, JSON.stringify(data));
      this.logger.debug(`Session set: ${sessionId}`);
    } catch (error) {
      this.logger.error(`Error setting session ${sessionId}: ${error.message}`);
      throw error;
    }
  }

  /**
   * Obtiene una sesión
   */
  async getSession<T = any>(sessionId: string): Promise<T | null> {
    const key = `${this.sessionPrefix}${sessionId}`;
    
    try {
      const data = await this.redis.get(key);
      if (!data) {
        return null;
      }
      return JSON.parse(data) as T;
    } catch (error) {
      this.logger.error(`Error getting session ${sessionId}: ${error.message}`);
      return null;
    }
  }

  /**
   * Elimina una sesión
   */
  async deleteSession(sessionId: string): Promise<void> {
    const key = `${this.sessionPrefix}${sessionId}`;
    
    try {
      await this.redis.del(key);
      this.logger.debug(`Session deleted: ${sessionId}`);
    } catch (error) {
      this.logger.error(`Error deleting session ${sessionId}: ${error.message}`);
    }
  }

  /**
   * Extiende el TTL de una sesión
   */
  async extendSession(sessionId: string, ttl: number = this.defaultTtl): Promise<boolean> {
    const key = `${this.sessionPrefix}${sessionId}`;
    
    try {
      const result = await this.redis.expire(key, ttl);
      return result === 1;
    } catch (error) {
      this.logger.error(`Error extending session ${sessionId}: ${error.message}`);
      return false;
    }
  }

  /**
   * Verifica si una sesión existe
   */
  async sessionExists(sessionId: string): Promise<boolean> {
    const key = `${this.sessionPrefix}${sessionId}`;
    
    try {
      const result = await this.redis.exists(key);
      return result === 1;
    } catch (error) {
      this.logger.error(`Error checking session ${sessionId}: ${error.message}`);
      return false;
    }
  }

  /**
   * Obtiene el TTL restante de una sesión
   */
  async getSessionTtl(sessionId: string): Promise<number> {
    const key = `${this.sessionPrefix}${sessionId}`;
    
    try {
      return await this.redis.ttl(key);
    } catch (error) {
      this.logger.error(`Error getting session TTL ${sessionId}: ${error.message}`);
      return -1;
    }
  }

  /**
   * Elimina todas las sesiones de un usuario
   */
  async deleteUserSessions(userId: string): Promise<number> {
    try {
      const pattern = `${this.sessionPrefix}*:${userId}`;
      const keys = await this.redis.keys(pattern);
      
      if (keys.length === 0) {
        return 0;
      }

      const deleted = await this.redis.del(...keys);
      this.logger.debug(`Deleted ${deleted} sessions for user ${userId}`);
      return deleted;
    } catch (error) {
      this.logger.error(`Error deleting user sessions ${userId}: ${error.message}`);
      return 0;
    }
  }
}
