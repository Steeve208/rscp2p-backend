import { Injectable, Inject, Logger } from '@nestjs/common';
import { Redis } from 'ioredis';

@Injectable()
export class RedisLockService {
  private readonly logger = new Logger(RedisLockService.name);
  private readonly defaultTtl = 300; // 5 minutos por defecto

  constructor(
    @Inject('REDIS_CLIENT')
    private readonly redis: Redis,
  ) {}

  /**
   * Adquiere un lock para una orden
   * Retorna true si se adquirió el lock, false si ya está bloqueada
   */
  async acquireOrderLock(orderId: string, ttl: number = this.defaultTtl): Promise<boolean> {
    const key = `lock:order:${orderId}`;
    
    try {
      // SET con NX (only if Not eXists) y EX (expiration)
      const result = await this.redis.set(key, 'locked', 'EX', ttl, 'NX');
      return result === 'OK';
    } catch (error) {
      this.logger.error(`Error acquiring lock for order ${orderId}: ${error.message}`);
      // Si Redis falla, permitir operación (no crítico)
      return true;
    }
  }

  /**
   * Libera un lock de una orden
   */
  async releaseOrderLock(orderId: string): Promise<void> {
    const key = `lock:order:${orderId}`;
    
    try {
      await this.redis.del(key);
      this.logger.debug(`Lock released for order ${orderId}`);
    } catch (error) {
      this.logger.error(`Error releasing lock for order ${orderId}: ${error.message}`);
    }
  }

  /**
   * Verifica si una orden está bloqueada
   */
  async isOrderLocked(orderId: string): Promise<boolean> {
    const key = `lock:order:${orderId}`;
    
    try {
      const result = await this.redis.exists(key);
      return result === 1;
    } catch (error) {
      this.logger.error(`Error checking lock for order ${orderId}: ${error.message}`);
      return false;
    }
  }

  /**
   * Extiende el TTL de un lock
   */
  async extendOrderLock(orderId: string, ttl: number = this.defaultTtl): Promise<boolean> {
    const key = `lock:order:${orderId}`;
    
    try {
      const result = await this.redis.expire(key, ttl);
      return result === 1;
    } catch (error) {
      this.logger.error(`Error extending lock for order ${orderId}: ${error.message}`);
      return false;
    }
  }

  /**
   * Adquiere un lock genérico
   */
  async acquireLock(lockKey: string, ttl: number = this.defaultTtl): Promise<boolean> {
    const key = `lock:${lockKey}`;
    
    try {
      const result = await this.redis.set(key, 'locked', 'EX', ttl, 'NX');
      return result === 'OK';
    } catch (error) {
      this.logger.error(`Error acquiring lock ${lockKey}: ${error.message}`);
      return true;
    }
  }

  /**
   * Libera un lock genérico
   */
  async releaseLock(lockKey: string): Promise<void> {
    const key = `lock:${lockKey}`;
    
    try {
      await this.redis.del(key);
    } catch (error) {
      this.logger.error(`Error releasing lock ${lockKey}: ${error.message}`);
    }
  }

  /**
   * Ejecuta una función con lock (patrón try-finally)
   */
  async withLock<T>(
    lockKey: string,
    fn: () => Promise<T>,
    ttl: number = this.defaultTtl,
  ): Promise<T> {
    const acquired = await this.acquireLock(lockKey, ttl);
    
    if (!acquired) {
      throw new Error(`Could not acquire lock: ${lockKey}`);
    }

    try {
      return await fn();
    } finally {
      await this.releaseLock(lockKey);
    }
  }
}
