import { Injectable, Logger, OnModuleInit } from '@nestjs/common';
import { JobTrackerService } from './job-tracker.service';

@Injectable()
export class JobRecoveryService implements OnModuleInit {
  private readonly logger = new Logger(JobRecoveryService.name);

  constructor(private readonly jobTracker: JobTrackerService) {}

  /**
   * Se ejecuta al iniciar el módulo.
   * Recupera jobs que estaban en ejecución antes del reinicio (libera locks huérfanos).
   */
  async onModuleInit() {
    this.logger.log('Iniciando recuperación de jobs...');

    try {
      const criticalJobs = ['expired-orders-cleanup', 'consistency-check'];

      for (const jobName of criticalJobs) {
        const isRunning = await this.jobTracker.isJobRunning(jobName);

        if (isRunning) {
          this.logger.warn(
            `Job ${jobName} tiene lock activo, posible reinicio durante ejecución`,
          );
          await this.jobTracker.releaseJobLock(jobName);
          this.logger.log(`Lock liberado para job ${jobName}`);
        }

        const savedState = await this.jobTracker.getJobState(jobName);
        if (savedState) {
          this.logger.log(
            `Estado guardado encontrado para ${jobName}: ${JSON.stringify(savedState)}`,
          );
        }
      }

      this.logger.log('Recuperación de jobs completada');
    } catch (error) {
      this.logger.error(
        `Error en recuperación de jobs: ${error.message}`,
        error.stack,
      );
    }
  }
}
