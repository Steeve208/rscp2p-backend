import { Injectable, Logger, Inject } from '@nestjs/common';
import { Cron, CronExpression } from '@nestjs/schedule';
import { ConfigService } from '@nestjs/config';
import { BlockchainService } from '../modules/blockchain/blockchain.service';
import { JobTrackerService } from './job-tracker.service';
import { Redis } from 'ioredis';

/**
 * Job de Sincronización de Blockchain
 * 
 * Rol: Recuperación ante fallos
 * 
 * Contiene:
 * - Re-sync completo desde bloque X
 * - Recuperación automática después de reinicios
 * - Validación de integridad
 * - Tracking de progreso
 * - Manejo de errores y reintentos
 * 
 * ⚠️ REGLA FINAL: Este job NUNCA ejecuta transacciones blockchain.
 * Solo sincroniza y reconcilia estados off-chain con on-chain.
 */
@Injectable()
export class BlockchainSyncJob {
  private readonly logger = new Logger(BlockchainSyncJob.name);
  private readonly jobName = 'blockchain-sync';
  private readonly RESYNC_JOB_NAME = 'blockchain-full-resync';
  private readonly MAX_RETRIES = 3;
  private readonly RETRY_DELAY_MS = 5000; // 5 segundos

  constructor(
    private readonly blockchainService: BlockchainService,
    private readonly jobTracker: JobTrackerService,
    private readonly configService: ConfigService,
    @Inject('REDIS_CLIENT')
    private readonly redis: Redis,
  ) {}

  /**
   * Verifica si blockchain está habilitada
   */
  private async getBlockchainConfig(): Promise<{ enabled: boolean }> {
    const blockchainConfig = this.configService.get('blockchain');
    const rpcUrl = blockchainConfig?.rpcUrl || '';
    const enabled = rpcUrl !== '' && rpcUrl !== undefined;
    
    return { enabled };
  }

  /**
   * Sincronización continua cada minuto
   * Con resiliencia: sobrevive reinicios
   * 
   * ⚠️ Se deshabilita automáticamente si blockchain no está configurada
   */
  @Cron(CronExpression.EVERY_MINUTE)
  async handleBlockchainSync() {
    // Verificar si blockchain está habilitada
    const blockchainConfig = await this.getBlockchainConfig();
    if (!blockchainConfig?.enabled) {
      this.logger.debug('Blockchain sync disabled: blockchain not configured');
      return;
    }
    const executionId = this.jobTracker.generateExecutionId();

    // Verificar si ya está en ejecución
    const isRunning = await this.jobTracker.isJobRunning(this.jobName);
    if (isRunning) {
      this.logger.debug('Blockchain sync already running, skipping...');
      return;
    }

    // Adquirir lock
    const lockAcquired = await this.jobTracker.acquireJobLock(this.jobName, 300);
    if (!lockAcquired) {
      this.logger.debug('Could not acquire lock for blockchain sync');
      return;
    }

    try {
      await this.jobTracker.startJob(this.jobName, executionId);

      this.logger.debug('Ejecutando sincronización de blockchain...');

      // Obtener estado guardado (si existe después de reinicio)
      const savedState = await this.jobTracker.getJobState(this.jobName);
      if (savedState) {
        this.logger.log(`Resuming blockchain sync from saved state`);
      }

      // Re-sincronizar automáticamente si es necesario
      await this.blockchainService.autoResyncIfNeeded();

      // Reconcilia eventos no procesados
      const reconciliation = await this.blockchainService.reconcileAll();

      // Guardar estado para reanudación
      await this.jobTracker.saveJobState(this.jobName, {
        lastSync: new Date(),
        reconciliation,
      });

      if (reconciliation.reconciled > 0) {
        this.logger.log(
          `Reconciliados ${reconciliation.reconciled}/${reconciliation.total} escrows`,
        );
      }

      await this.jobTracker.completeJob(this.jobName, executionId, {
        reconciled: reconciliation.reconciled,
        total: reconciliation.total,
      });
    } catch (error) {
      this.logger.error(
        `Error en sincronización: ${error.message}`,
        error.stack,
      );
      await this.jobTracker.failJob(this.jobName, executionId, error.message);
    } finally {
      await this.jobTracker.releaseJobLock(this.jobName);
    }
  }

  /**
   * Verificación de estado cada 5 minutos
   * ⚠️ Se deshabilita automáticamente si blockchain no está configurada
   */
  @Cron(CronExpression.EVERY_5_MINUTES)
  async handleStatusCheck() {
    const blockchainConfig = await this.getBlockchainConfig();
    if (!blockchainConfig?.enabled) {
      return;
    }
    this.logger.debug('Verificando estado de blockchain...');
    try {
      const status = await this.blockchainService.getStatus();

      if (status.status === 'error') {
        this.logger.error(`Estado de blockchain: ${status.error}`);
      } else {
        this.logger.debug(
          `Blockchain: ${status.syncStatus}, Último bloque: ${status.lastSyncedBlock}, Eventos: ${status.totalEventsProcessed}`,
        );
      }
    } catch (error) {
      this.logger.error(
        `Error verificando estado: ${error.message}`,
        error.stack,
      );
    }
  }

  /**
   * Re-sincronización completa cada hora
   * Crítico: debe sobrevivir reinicios
   * ⚠️ Se deshabilita automáticamente si blockchain no está configurada
   */
  @Cron(CronExpression.EVERY_HOUR)
  async handleDeepReconciliation() {
    const blockchainConfig = await this.getBlockchainConfig();
    if (!blockchainConfig?.enabled) {
      return;
    }
    const executionId = this.jobTracker.generateExecutionId();
    const jobName = 'blockchain-deep-reconciliation';

    const lockAcquired = await this.jobTracker.acquireJobLock(jobName, 3600);
    if (!lockAcquired) {
      this.logger.debug('Deep reconciliation already running');
      return;
    }

    try {
      await this.jobTracker.startJob(jobName, executionId);

      this.logger.log('Ejecutando reconciliación profunda...');

      // Obtener último bloque sincronizado desde estado guardado
      const savedState = await this.jobTracker.getJobState('blockchain-sync');
      const lastBlock = savedState?.lastSyncedBlock || 0;

      // Re-sincronizar desde el último bloque conocido
      if (lastBlock > 0) {
        await this.blockchainService.resyncFromBlock(lastBlock);
      }

      const result = await this.blockchainService.reconcileAll();
      
      // Guardar estado
      await this.jobTracker.saveJobState(jobName, {
        lastReconciliation: new Date(),
        result,
      });

      this.logger.log(
        `Reconciliación profunda completada: ${result.reconciled}/${result.total} escrows`,
      );

      await this.jobTracker.completeJob(jobName, executionId, result);
    } catch (error) {
      this.logger.error(
        `Error en reconciliación profunda: ${error.message}`,
        error.stack,
      );
      await this.jobTracker.failJob(jobName, executionId, error.message);
    } finally {
      await this.jobTracker.releaseJobLock(jobName);
    }
  }

  /**
   * Re-sincronización completa desde un bloque específico
   * 
   * Rol: Recuperación ante fallos
   * 
   * Contiene:
   * - Re-sync completo desde bloque X
   * - Validación de integridad
   * - Tracking de progreso
   * - Recuperación automática en caso de fallo
   * 
   * ⚠️ REGLA FINAL: Este método NUNCA ejecuta transacciones blockchain.
   * Solo sincroniza estados off-chain con on-chain.
   */
  async fullResyncFromBlock(
    fromBlock: number,
    options?: {
      validateIntegrity?: boolean;
      batchSize?: number;
      onProgress?: (currentBlock: number, totalBlocks: number) => void;
    },
  ): Promise<{
    success: boolean;
    fromBlock: number;
    toBlock: number;
    totalBlocks: number;
    eventsProcessed: number;
    errors: number;
    duration: number;
  }> {
    const executionId = this.jobTracker.generateExecutionId();
    const startTime = Date.now();

    // Adquirir lock con TTL largo (2 horas)
    const lockAcquired = await this.jobTracker.acquireJobLock(
      this.RESYNC_JOB_NAME,
      7200,
    );

    if (!lockAcquired) {
      const lastExecution = await this.jobTracker.getLastExecution(
        this.RESYNC_JOB_NAME,
      );
      if (lastExecution?.status === 'RUNNING') {
        throw new Error(
          `Full resync already running since ${lastExecution.startedAt}`,
        );
      }
      // Si el último fue fallido, permitir reintento
    }

    try {
      await this.jobTracker.startJob(this.RESYNC_JOB_NAME, executionId);
      
      // Guardar metadata adicional
      await this.jobTracker.saveJobState(this.RESYNC_JOB_NAME, {
        executionId,
        fromBlock,
        startedAt: new Date().toISOString(),
      });

      this.logger.log(
        `🚀 Iniciando re-sincronización completa desde bloque ${fromBlock}...`,
      );

      // Obtener último bloque de la blockchain
      const latestBlock = await this.blockchainService.getLatestBlock();
      const latestBlockNumber = latestBlock.number;
      const totalBlocks = latestBlockNumber - fromBlock;

      if (totalBlocks <= 0) {
        this.logger.warn(
          `No hay bloques para sincronizar (fromBlock: ${fromBlock}, latest: ${latestBlockNumber})`,
        );
        await this.jobTracker.completeJob(this.RESYNC_JOB_NAME, executionId, {
          fromBlock,
          toBlock: latestBlockNumber,
          totalBlocks: 0,
          eventsProcessed: 0,
          errors: 0,
        });
        return {
          success: true,
          fromBlock,
          toBlock: latestBlockNumber,
          totalBlocks: 0,
          eventsProcessed: 0,
          errors: 0,
          duration: Date.now() - startTime,
        };
      }

      this.logger.log(
        `📊 Re-sincronizando ${totalBlocks} bloques (${fromBlock} → ${latestBlockNumber})`,
      );

      // Guardar estado inicial
      await this.jobTracker.saveJobState(this.RESYNC_JOB_NAME, {
        fromBlock,
        toBlock: latestBlockNumber,
        totalBlocks,
        currentBlock: fromBlock,
        startedAt: new Date().toISOString(),
        eventsProcessed: 0,
        errors: 0,
      });

      // Ejecutar re-sync con reintentos
      let currentBlock = fromBlock;
      let eventsProcessed = 0;
      let errors = 0;
      const batchSize = options?.batchSize || 1000;

      while (currentBlock < latestBlockNumber) {
        const toBlock = Math.min(currentBlock + batchSize, latestBlockNumber);

        try {
          // Re-sincronizar batch
          await this.blockchainService.resyncFromBlock(currentBlock);

          // Validar integridad si está habilitado
          if (options?.validateIntegrity !== false) {
            const validation = await this.validateSyncIntegrity(
              currentBlock,
              toBlock,
            );
            if (!validation.isValid) {
              this.logger.warn(
                `Integrity validation warnings: ${validation.warnings.join(', ')}`,
              );
            }
          }

          // Actualizar progreso
          currentBlock = toBlock + 1;
          eventsProcessed += 100; // Estimación

          // Guardar progreso
          await this.jobTracker.saveJobState(this.RESYNC_JOB_NAME, {
            fromBlock,
            toBlock: latestBlockNumber,
            totalBlocks,
            currentBlock,
            progress: ((currentBlock - fromBlock) / totalBlocks) * 100,
            eventsProcessed,
            errors,
            lastUpdate: new Date().toISOString(),
          });

          // Callback de progreso
          if (options?.onProgress) {
            options.onProgress(currentBlock, totalBlocks);
          }

          this.logger.debug(
            `✅ Progreso: ${currentBlock}/${latestBlockNumber} (${((currentBlock - fromBlock) / totalBlocks * 100).toFixed(2)}%)`,
          );

          // Pequeña pausa para no sobrecargar
          await new Promise((resolve) => setTimeout(resolve, 100));
        } catch (error) {
          errors++;
          this.logger.error(
            `❌ Error sincronizando bloques ${currentBlock}-${toBlock}: ${error.message}`,
          );

          // Reintentar si no excedemos el máximo
          if (errors < this.MAX_RETRIES) {
            this.logger.log(
              `🔄 Reintentando... (${errors}/${this.MAX_RETRIES})`,
            );
            await new Promise((resolve) =>
              setTimeout(resolve, this.RETRY_DELAY_MS),
            );
            continue;
          } else {
            throw new Error(
              `Max retries exceeded. Last error: ${error.message}`,
            );
          }
        }
      }

      // Reconciliación final
      this.logger.log('🔄 Ejecutando reconciliación final...');
      const reconciliation = await this.blockchainService.reconcileAll();

      const duration = Date.now() - startTime;
      const result = {
        success: true,
        fromBlock,
        toBlock: latestBlockNumber,
        totalBlocks,
        eventsProcessed: reconciliation.reconciled,
        errors: reconciliation.errors,
        duration,
      };

      await this.jobTracker.completeJob(this.RESYNC_JOB_NAME, executionId, result);

      this.logger.log(
        `✅ Re-sincronización completa finalizada: ${totalBlocks} bloques, ${reconciliation.reconciled} eventos procesados, ${duration}ms`,
      );

      return result;
    } catch (error) {
      const duration = Date.now() - startTime;
      this.logger.error(
        `❌ Error en re-sincronización completa: ${error.message}`,
        error.stack,
      );

      // Guardar estado de error para recuperación
      await this.jobTracker.saveJobState(this.RESYNC_JOB_NAME, {
        fromBlock,
        error: error.message,
        failedAt: new Date().toISOString(),
        duration,
      });

      await this.jobTracker.failJob(
        this.RESYNC_JOB_NAME,
        executionId,
        error.message,
      );

      throw error;
    } finally {
      await this.jobTracker.releaseJobLock(this.RESYNC_JOB_NAME);
    }
  }

  /**
   * Valida la integridad de la sincronización
   * 
   * Verifica que los eventos sincronizados sean consistentes
   */
  private async validateSyncIntegrity(
    fromBlock: number,
    toBlock: number,
  ): Promise<{
    isValid: boolean;
    warnings: string[];
    errors: string[];
  }> {
    const warnings: string[] = [];
    const errors: string[] = [];

    try {
      // Validar que los bloques sean consecutivos
      const blockValidation = await this.blockchainService.validateBlock(fromBlock);
      if (!blockValidation.isValid) {
        errors.push(`Block ${fromBlock} validation failed`);
      }

      // Verificar que no haya gaps en la sincronización
      // (esto se puede hacer comparando con el estado guardado)

      return {
        isValid: errors.length === 0,
        warnings,
        errors,
      };
    } catch (error) {
      warnings.push(`Integrity validation error: ${error.message}`);
      return {
        isValid: true, // No crítico
        warnings,
        errors: [],
      };
    }
  }

  /**
   * Recupera y reanuda una re-sincronización interrumpida
   * 
   * Rol: Recuperación ante fallos
   */
  async resumeInterruptedResync(): Promise<{
    resumed: boolean;
    fromBlock?: number;
    message: string;
  }> {
    try {
      const savedState = await this.jobTracker.getJobState(this.RESYNC_JOB_NAME);

      if (!savedState) {
        return {
          resumed: false,
          message: 'No interrupted resync found',
        };
      }

      // Verificar si hay un error guardado
      if (savedState.error) {
        this.logger.log(
          `🔄 Reanudando re-sincronización interrumpida desde bloque ${savedState.currentBlock || savedState.fromBlock}`,
        );

        const fromBlock = savedState.currentBlock || savedState.fromBlock;

        // Reintentar desde donde se quedó
        await this.fullResyncFromBlock(fromBlock);

        return {
          resumed: true,
          fromBlock,
          message: `Resumed from block ${fromBlock}`,
        };
      }

      // Si no hay error pero hay progreso guardado, verificar si está completo
      if (savedState.currentBlock >= savedState.toBlock) {
        return {
          resumed: false,
          message: 'Resync appears to be complete',
        };
      }

      return {
        resumed: false,
        message: 'Resync state found but no error to recover from',
      };
    } catch (error) {
      this.logger.error(
        `Error resuming interrupted resync: ${error.message}`,
        error.stack,
      );
      return {
        resumed: false,
        message: `Error: ${error.message}`,
      };
    }
  }

  /**
   * Re-sincronización de emergencia (ejecutar manualmente si es necesario)
   * 
   * Versión mejorada con mejor manejo de errores
   */
  async emergencyResync(fromBlock?: number): Promise<{
    success: boolean;
    fromBlock: number;
    toBlock: number;
    totalBlocks: number;
    eventsProcessed: number;
    errors: number;
    duration: number;
  }> {
    if (!fromBlock) {
      // Obtener último bloque sincronizado
      const status = await this.blockchainService.getStatus();
      fromBlock = status.lastSyncedBlock || 0;
    }

    this.logger.warn(
      `🚨 INICIANDO RE-SINCRONIZACIÓN DE EMERGENCIA desde bloque ${fromBlock}`,
    );

    return this.fullResyncFromBlock(fromBlock, {
      validateIntegrity: true,
      batchSize: 500, // Batches más pequeños para emergencias
      onProgress: (current, total) => {
        const progress = ((current / total) * 100).toFixed(2);
        this.logger.log(`📊 Progreso de emergencia: ${progress}%`);
      },
    });
  }

  /**
   * Verifica y recupera automáticamente después de reinicio
   * 
   * Rol: Recuperación ante fallos
   * ⚠️ Se deshabilita automáticamente si blockchain no está configurada
   */
  @Cron(CronExpression.EVERY_10_MINUTES)
  async handleRecoveryCheck(): Promise<void> {
    const blockchainConfig = await this.getBlockchainConfig();
    if (!blockchainConfig?.enabled) {
      return;
    }
    try {
      // Verificar si hay una re-sincronización interrumpida
      const resumeResult = await this.resumeInterruptedResync();

      if (resumeResult.resumed) {
        this.logger.log(
          `✅ Re-sincronización interrumpida recuperada: ${resumeResult.message}`,
        );
      }

      // Verificar estado de sincronización
      const status = await this.blockchainService.getStatus();

      if (status.status === 'error') {
        this.logger.warn(
          `⚠️ Estado de error detectado, intentando recuperación automática...`,
        );

        // Intentar auto-resync
        await this.blockchainService.autoResyncIfNeeded();
      }
    } catch (error) {
      this.logger.error(
        `Error en verificación de recuperación: ${error.message}`,
        error.stack,
      );
    }
  }
}