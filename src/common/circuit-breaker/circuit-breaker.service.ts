import { Injectable, Logger } from '@nestjs/common';
import { Redis } from 'ioredis';
import { Inject } from '@nestjs/common';

/**
 * Circuit Breaker Service
 * 
 * Implementa el patrón Circuit Breaker para proteger contra fallos en cascada
 * de servicios externos (blockchain RPC, APIs externas, etc.)
 * 
 * Estados:
 * - CLOSED: Funcionando normalmente
 * - OPEN: Fallando, rechaza requests inmediatamente
 * - HALF_OPEN: Probando si el servicio se recuperó
 * 
 * Configuración:
 * - failureThreshold: Número de fallos antes de abrir
 * - timeout: Tiempo en OPEN antes de intentar HALF_OPEN
 * - successThreshold: Éxitos necesarios en HALF_OPEN para cerrar
 */
@Injectable()
export class CircuitBreakerService {
  private readonly logger = new Logger(CircuitBreakerService.name);
  private readonly circuits: Map<string, CircuitStateInternal> = new Map();

  constructor(
    @Inject('REDIS_CLIENT')
    private readonly redis: Redis,
  ) {}

  /**
   * Ejecuta una función con protección de circuit breaker
   */
  async execute<T>(
    circuitName: string,
    fn: () => Promise<T>,
    options?: CircuitBreakerOptions,
  ): Promise<T> {
    const circuit = this.getOrCreateCircuit(circuitName, options);
    const state = await this.getState(circuitName);

    // Si está OPEN, rechazar inmediatamente
    if (state === CircuitState.OPEN) {
      const canAttemptHalfOpen = await this.canAttemptHalfOpen(circuitName);
      if (!canAttemptHalfOpen) {
        throw new CircuitBreakerOpenError(
          `Circuit breaker ${circuitName} is OPEN`,
        );
      }
      // Intentar HALF_OPEN
      await this.setState(circuitName, CircuitState.HALF_OPEN);
    }

    try {
      const result = await fn();
      
      // Si está en HALF_OPEN, registrar éxito
      if (state === CircuitState.HALF_OPEN) {
        await this.recordSuccess(circuitName);
        const successCount = await this.getSuccessCount(circuitName);
        if (successCount >= (circuit.config.successThreshold || 1)) {
          await this.closeCircuit(circuitName);
        }
      } else {
        // Si está CLOSED, resetear contador de fallos
        await this.resetFailureCount(circuitName);
      }

      return result;
    } catch (error) {
      // Registrar fallo
      await this.recordFailure(circuitName);
      const failureCount = await this.getFailureCount(circuitName);

      // Si excede el threshold, abrir el circuito
      if (failureCount >= (circuit.config.failureThreshold || 5)) {
        await this.openCircuit(circuitName);
        this.logger.warn(
          `Circuit breaker ${circuitName} opened after ${failureCount} failures`,
        );
      }

      throw error;
    }
  }

  /**
   * Obtiene o crea un circuito
   */
  private getOrCreateCircuit(
    circuitName: string,
    options?: CircuitBreakerOptions,
  ): CircuitStateInternal {
    if (!this.circuits.has(circuitName)) {
      this.circuits.set(circuitName, {
        state: CircuitState.CLOSED,
        failureCount: 0,
        successCount: 0,
        lastFailureTime: null,
        openedAt: null,
        config: {
          failureThreshold: options?.failureThreshold || 5,
          timeout: options?.timeout || 60000, // 60 segundos
          successThreshold: options?.successThreshold || 1,
        },
      });
    }
    return this.circuits.get(circuitName)!;
  }

  /**
   * Obtiene el estado actual del circuito
   */
  private async getState(circuitName: string): Promise<CircuitState> {
    const stateKey = `circuit:${circuitName}:state`;
    const state = await this.redis.get(stateKey);
    return (state as CircuitState) || CircuitState.CLOSED;
  }

  /**
   * Establece el estado del circuito
   */
  private async setState(
    circuitName: string,
    state: CircuitState,
  ): Promise<void> {
    const stateKey = `circuit:${circuitName}:state`;
    await this.redis.set(stateKey, state);
    
    const circuit = this.circuits.get(circuitName);
    if (circuit) {
      circuit.state = state;
      if (state === CircuitState.OPEN) {
        circuit.openedAt = new Date();
      }
    }
  }

  /**
   * Registra un fallo
   */
  private async recordFailure(circuitName: string): Promise<void> {
    const failureKey = `circuit:${circuitName}:failures`;
    const count = await this.redis.incr(failureKey);
    await this.redis.expire(failureKey, 300); // 5 minutos

    const circuit = this.circuits.get(circuitName);
    if (circuit) {
      circuit.failureCount = count;
      circuit.lastFailureTime = new Date();
    }
  }

  /**
   * Obtiene el contador de fallos
   */
  private async getFailureCount(circuitName: string): Promise<number> {
    const failureKey = `circuit:${circuitName}:failures`;
    const count = await this.redis.get(failureKey);
    return count ? parseInt(count, 10) : 0;
  }

  /**
   * Resetea el contador de fallos
   */
  private async resetFailureCount(circuitName: string): Promise<void> {
    const failureKey = `circuit:${circuitName}:failures`;
    await this.redis.del(failureKey);

    const circuit = this.circuits.get(circuitName);
    if (circuit) {
      circuit.failureCount = 0;
    }
  }

  /**
   * Registra un éxito
   */
  private async recordSuccess(circuitName: string): Promise<void> {
    const successKey = `circuit:${circuitName}:successes`;
    const count = await this.redis.incr(successKey);
    await this.redis.expire(successKey, 300); // 5 minutos

    const circuit = this.circuits.get(circuitName);
    if (circuit) {
      circuit.successCount = count;
    }
  }

  /**
   * Obtiene el contador de éxitos
   */
  private async getSuccessCount(circuitName: string): Promise<number> {
    const successKey = `circuit:${circuitName}:successes`;
    const count = await this.redis.get(successKey);
    return count ? parseInt(count, 10) : 0;
  }

  /**
   * Abre el circuito
   */
  private async openCircuit(circuitName: string): Promise<void> {
    await this.setState(circuitName, CircuitState.OPEN);
    const openedAtKey = `circuit:${circuitName}:opened_at`;
    await this.redis.set(openedAtKey, Date.now().toString());
  }

  /**
   * Cierra el circuito
   */
  private async closeCircuit(circuitName: string): Promise<void> {
    await this.setState(circuitName, CircuitState.CLOSED);
    await this.resetFailureCount(circuitName);
    const successKey = `circuit:${circuitName}:successes`;
    await this.redis.del(successKey);
    const openedAtKey = `circuit:${circuitName}:opened_at`;
    await this.redis.del(openedAtKey);

    this.logger.log(`Circuit breaker ${circuitName} closed`);
  }

  /**
   * Verifica si puede intentar HALF_OPEN
   */
  private async canAttemptHalfOpen(circuitName: string): Promise<boolean> {
    const openedAtKey = `circuit:${circuitName}:opened_at`;
    const openedAt = await this.redis.get(openedAtKey);
    
    if (!openedAt) {
      return true; // Nunca se abrió, puede intentar
    }

    const circuit = this.circuits.get(circuitName);
    if (!circuit) {
      return true;
    }

    const timeout = circuit.config.timeout || 60000;
    const timeSinceOpen = Date.now() - parseInt(openedAt, 10);
    
    return timeSinceOpen >= timeout;
  }

  /**
   * Obtiene el estado de un circuito
   */
  async getCircuitStatus(circuitName: string): Promise<{
    state: CircuitState;
    failureCount: number;
    successCount: number;
    openedAt: Date | null;
  }> {
    const state = await this.getState(circuitName);
    const failureCount = await this.getFailureCount(circuitName);
    const successCount = await this.getSuccessCount(circuitName);
    
    const circuit = this.circuits.get(circuitName);
    const openedAt = circuit?.openedAt || null;

    return {
      state,
      failureCount,
      successCount,
      openedAt,
    };
  }

  /**
   * Resetea manualmente un circuito (para administración)
   */
  async resetCircuit(circuitName: string): Promise<void> {
    await this.closeCircuit(circuitName);
    this.logger.log(`Circuit breaker ${circuitName} manually reset`);
  }
}

/**
 * Estados del circuit breaker
 */
export enum CircuitState {
  CLOSED = 'CLOSED',
  OPEN = 'OPEN',
  HALF_OPEN = 'HALF_OPEN',
}

/**
 * Configuración del circuit breaker
 */
export interface CircuitBreakerOptions {
  failureThreshold?: number; // Fallos antes de abrir (default: 5)
  timeout?: number; // Tiempo en OPEN antes de HALF_OPEN (default: 60000ms)
  successThreshold?: number; // Éxitos en HALF_OPEN para cerrar (default: 1)
}

/**
 * Configuración interna del circuito
 */
interface CircuitConfig {
  failureThreshold: number;
  timeout: number;
  successThreshold: number;
}

/**
 * Estado interno del circuito
 */
interface CircuitStateInternal {
  state: CircuitState;
  failureCount: number;
  successCount: number;
  lastFailureTime: Date | null;
  openedAt: Date | null;
  config: CircuitConfig;
}

/**
 * Error cuando el circuit breaker está abierto
 */
export class CircuitBreakerOpenError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'CircuitBreakerOpenError';
  }
}

