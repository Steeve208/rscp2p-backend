import { Injectable, Logger, OnModuleInit } from '@nestjs/common';
import { JobTrackerService } from './job-tracker.service';
import { BlockchainService } from '../modules/blockchain/blockchain.service';

@Injectable()
export class JobRecoveryService implements OnModuleInit {
  private readonly logger = new Logger(JobRecoveryService.name);

  constructor(
    private readonly jobTracker: JobTrackerService,
    private readonly blockchainService: BlockchainService,
  ) {}

  /**
   * Se ejecuta al iniciar el módulo
   * Recupera jobs que estaban en ejecución antes del reinicio
   */
  async onModuleInit() {
    this.logger.log('Iniciando recuperación de jobs...');
    
    try {
      // Verificar si hay jobs bloqueados (posible reinicio durante ejecución)
      const criticalJobs = [
        'blockchain-sync',
        'blockchain-deep-reconciliation',
        'expired-orders-cleanup',
        'consistency-check',
      ];

      for (const jobName of criticalJobs) {
        const isRunning = await this.jobTracker.isJobRunning(jobName);
        
        if (isRunning) {
          this.logger.warn(
            `Job ${jobName} tiene lock activo, posible reinicio durante ejecución`,
          );
          
          // Liberar locks huérfanos (de reinicios anteriores)
          await this.jobTracker.releaseJobLock(jobName);
          this.logger.log(`Lock liberado para job ${jobName}`);
        }

        // Obtener último estado guardado
        const savedState = await this.jobTracker.getJobState(jobName);
        if (savedState) {
          this.logger.log(
            `Estado guardado encontrado para ${jobName}: ${JSON.stringify(savedState)}`,
          );
        }
      }

      // Recuperar sincronización de blockchain si es necesario
      await this.recoverBlockchainSync();

      this.logger.log('Recuperación de jobs completada');
    } catch (error) {
      this.logger.error(
        `Error en recuperación de jobs: ${error.message}`,
        error.stack,
      );
    }
  }

  /**
   * Recupera la sincronización de blockchain después de reinicio
   */
  private async recoverBlockchainSync(): Promise<void> {
    try {
      const savedState = await this.jobTracker.getJobState('blockchain-sync');
      
      if (savedState && savedState.lastSync) {
        const lastSync = new Date(savedState.lastSync);
        const hoursSinceLastSync = (Date.now() - lastSync.getTime()) / (1000 * 60 * 60);

        // Si pasó más de 1 hora desde la última sincronización, re-sincronizar
        if (hoursSinceLastSync > 1) {
          this.logger.log(
            `Última sincronización hace ${hoursSinceLastSync.toFixed(2)} horas, iniciando re-sincronización...`,
          );
          
          // Re-sincronizar automáticamente
          await this.blockchainService.autoResyncIfNeeded();
        }
      } else {
        // Primera vez, iniciar sincronización
        this.logger.log('Primera ejecución, iniciando sincronización de blockchain...');
        await this.blockchainService.autoResyncIfNeeded();
      }
    } catch (error) {
      this.logger.error(
        `Error recuperando sincronización de blockchain: ${error.message}`,
      );
    }
  }
}
