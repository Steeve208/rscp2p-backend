import { Injectable, Inject, Logger } from '@nestjs/common';
import { Redis } from 'ioredis';

export interface JobExecution {
  jobName: string;
  startedAt: Date;
  completedAt?: Date;
  status: 'RUNNING' | 'COMPLETED' | 'FAILED';
  error?: string;
  result?: any;
}

@Injectable()
export class JobTrackerService {
  private readonly logger = new Logger(JobTrackerService.name);
  private readonly prefix = 'job:execution:';
  private readonly lockPrefix = 'job:lock:';

  constructor(
    @Inject('REDIS_CLIENT')
    private readonly redis: Redis,
  ) {}

  /**
   * Inicia el tracking de un job
   */
  async startJob(jobName: string, executionId: string): Promise<void> {
    const key = `${this.prefix}${jobName}:${executionId}`;
    const execution: JobExecution = {
      jobName,
      startedAt: new Date(),
      status: 'RUNNING',
    };

    try {
      await this.redis.setex(
        key,
        86400, // 24 horas
        JSON.stringify(execution),
      );
      this.logger.debug(`Job started: ${jobName} (${executionId})`);
    } catch (error) {
      this.logger.error(`Error tracking job start: ${error.message}`);
    }
  }

  /**
   * Marca un job como completado
   */
  async completeJob(
    jobName: string,
    executionId: string,
    result?: any,
  ): Promise<void> {
    const key = `${this.prefix}${jobName}:${executionId}`;
    
    try {
      const data = await this.redis.get(key);
      if (data) {
        const execution: JobExecution = JSON.parse(data);
        execution.status = 'COMPLETED';
        execution.completedAt = new Date();
        execution.result = result;
        await this.redis.setex(key, 86400, JSON.stringify(execution));
        this.logger.debug(`Job completed: ${jobName} (${executionId})`);
      }
    } catch (error) {
      this.logger.error(`Error tracking job completion: ${error.message}`);
    }
  }

  /**
   * Marca un job como fallido
   */
  async failJob(
    jobName: string,
    executionId: string,
    error: string,
  ): Promise<void> {
    const key = `${this.prefix}${jobName}:${executionId}`;
    
    try {
      const data = await this.redis.get(key);
      if (data) {
        const execution: JobExecution = JSON.parse(data);
        execution.status = 'FAILED';
        execution.completedAt = new Date();
        execution.error = error;
        await this.redis.setex(key, 86400, JSON.stringify(execution));
        this.logger.error(`Job failed: ${jobName} (${executionId}) - ${error}`);
      }
    } catch (error) {
      this.logger.error(`Error tracking job failure: ${error.message}`);
    }
  }

  /**
   * Adquiere un lock para un job (previene ejecuciones concurrentes)
   */
  async acquireJobLock(jobName: string, ttl: number = 3600): Promise<boolean> {
    const key = `${this.lockPrefix}${jobName}`;
    
    try {
      const result = await this.redis.set(key, 'locked', 'EX', ttl, 'NX');
      return result === 'OK';
    } catch (error) {
      this.logger.error(`Error acquiring job lock: ${error.message}`);
      return false;
    }
  }

  /**
   * Libera el lock de un job
   */
  async releaseJobLock(jobName: string): Promise<void> {
    const key = `${this.lockPrefix}${jobName}`;
    
    try {
      await this.redis.del(key);
    } catch (error) {
      this.logger.error(`Error releasing job lock: ${error.message}`);
    }
  }

  /**
   * Verifica si un job está en ejecución
   */
  async isJobRunning(jobName: string): Promise<boolean> {
    const key = `${this.lockPrefix}${jobName}`;
    
    try {
      const result = await this.redis.exists(key);
      return result === 1;
    } catch (error) {
      return false;
    }
  }

  /**
   * Obtiene el último estado de un job
   */
  async getLastExecution(jobName: string): Promise<JobExecution | null> {
    try {
      const pattern = `${this.prefix}${jobName}:*`;
      const keys = await this.redis.keys(pattern);
      
      if (keys.length === 0) {
        return null;
      }

      // Obtener la ejecución más reciente
      const executions = await Promise.all(
        keys.map(async (key) => {
          const data = await this.redis.get(key);
          return data ? JSON.parse(data) : null;
        }),
      );

      const validExecutions = executions
        .filter((e) => e !== null)
        .sort((a, b) => new Date(b.startedAt).getTime() - new Date(a.startedAt).getTime());

      return validExecutions[0] || null;
    } catch (error) {
      this.logger.error(`Error getting last execution: ${error.message}`);
      return null;
    }
  }

  /**
   * Guarda el estado de un job para reanudación después de reinicio
   */
  async saveJobState(jobName: string, state: any, ttl: number = 86400): Promise<void> {
    const key = `job:state:${jobName}`;
    
    try {
      await this.redis.setex(
        key,
        ttl,
        JSON.stringify({
          ...state,
          savedAt: new Date(),
        }),
      );
    } catch (error) {
      this.logger.error(`Error saving job state: ${error.message}`);
    }
  }

  /**
   * Obtiene el estado guardado de un job
   */
  async getJobState<T = any>(jobName: string): Promise<T | null> {
    const key = `job:state:${jobName}`;
    
    try {
      const data = await this.redis.get(key);
      return data ? JSON.parse(data) : null;
    } catch (error) {
      this.logger.error(`Error getting job state: ${error.message}`);
      return null;
    }
  }

  /**
   * Genera un ID único para ejecución de job
   */
  generateExecutionId(): string {
    return `${Date.now()}-${Math.random().toString(36).substring(7)}`;
  }
}
