import { Injectable, Logger, Inject } from '@nestjs/common';
import { Cron, CronExpression } from '@nestjs/schedule';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository, LessThan } from 'typeorm';
import { Order } from '../database/entities/order.entity';
import { Notification } from '../database/entities/notification.entity';
import { OrderStatus } from '../common/enums/order-status.enum';
import { JobTrackerService } from './job-tracker.service';
import { NotificationsService } from '../modules/notifications/notifications.service';
import { OrdersService } from '../modules/orders/orders.service';

@Injectable()
export class CleanupJob {
  private readonly logger = new Logger(CleanupJob.name);

  constructor(
    @InjectRepository(Order)
    private readonly orderRepository: Repository<Order>,
    @InjectRepository(Notification)
    private readonly notificationRepository: Repository<Notification>,
    private readonly jobTracker: JobTrackerService,
    private readonly notificationsService: NotificationsService,
    private readonly ordersService: OrdersService,
  ) {}

  /**
   * Limpieza de órdenes expiradas cada hora
   * Crítico: debe sobrevivir reinicios
   */
  @Cron(CronExpression.EVERY_HOUR)
  async handleExpiredOrdersCleanup() {
    const executionId = this.jobTracker.generateExecutionId();
    const jobName = 'expired-orders-cleanup';

    const lockAcquired = await this.jobTracker.acquireJobLock(jobName, 3600);
    if (!lockAcquired) {
      this.logger.debug('Expired orders cleanup already running');
      return;
    }

    try {
      await this.jobTracker.startJob(jobName, executionId);

      this.logger.log('Iniciando limpieza de órdenes expiradas...');

      const now = new Date();
      const expiredOrders = await this.orderRepository.find({
        where: {
          status: OrderStatus.CREATED,
          expiresAt: LessThan(now),
        },
      });

      let cancelled = 0;
      let errors = 0;

      for (const order of expiredOrders) {
        try {
          // Cancelar orden expirada (como vendedor)
          await this.ordersService.cancel(order.id, order.sellerId);
          cancelled++;

          // Notificar al vendedor
          await this.notificationsService.notifyOrderStatusChange(
            order.sellerId,
            order.id,
            OrderStatus.REFUNDED,
            { reason: 'Orden expirada automáticamente' },
          );

          // Si hay comprador, notificar también
          if (order.buyerId) {
            await this.notificationsService.notifyOrderStatusChange(
              order.buyerId,
              order.id,
              OrderStatus.REFUNDED,
              { reason: 'Orden expirada automáticamente' },
            );
          }
        } catch (error) {
          this.logger.error(
            `Error cancelando orden expirada ${order.id}: ${error.message}`,
          );
          errors++;
        }
      }

      // Guardar estado
      await this.jobTracker.saveJobState(jobName, {
        lastCleanup: now,
        cancelled,
        errors,
        totalExpired: expiredOrders.length,
      });

      this.logger.log(
        `Limpieza completada: ${cancelled} órdenes canceladas, ${errors} errores`,
      );

      await this.jobTracker.completeJob(jobName, executionId, {
        cancelled,
        errors,
        total: expiredOrders.length,
      });
    } catch (error) {
      this.logger.error(
        `Error en limpieza de órdenes expiradas: ${error.message}`,
        error.stack,
      );
      await this.jobTracker.failJob(jobName, executionId, error.message);
    } finally {
      await this.jobTracker.releaseJobLock(jobName);
    }
  }

  /**
   * Limpieza de notificaciones antiguas cada día
   */
  @Cron(CronExpression.EVERY_DAY_AT_2AM)
  async handleNotificationsCleanup() {
    const executionId = this.jobTracker.generateExecutionId();
    const jobName = 'notifications-cleanup';

    const lockAcquired = await this.jobTracker.acquireJobLock(jobName, 3600);
    if (!lockAcquired) {
      return;
    }

    try {
      await this.jobTracker.startJob(jobName, executionId);

      this.logger.log('Iniciando limpieza de notificaciones...');

      const thirtyDaysAgo = new Date();
      thirtyDaysAgo.setDate(thirtyDaysAgo.getDate() - 30);

      const result = await this.notificationRepository
        .createQueryBuilder()
        .delete()
        .where('created_at < :date', { date: thirtyDaysAgo })
        .andWhere('read = :read', { read: true })
        .execute();

      const deleted = result.affected || 0;

      await this.jobTracker.saveJobState(jobName, {
        lastCleanup: new Date(),
        deleted,
      });

      this.logger.log(`Limpieza de notificaciones: ${deleted} eliminadas`);

      await this.jobTracker.completeJob(jobName, executionId, { deleted });
    } catch (error) {
      this.logger.error(
        `Error en limpieza de notificaciones: ${error.message}`,
        error.stack,
      );
      await this.jobTracker.failJob(jobName, executionId, error.message);
    } finally {
      await this.jobTracker.releaseJobLock(jobName);
    }
  }

  /**
   * Limpieza semanal de datos antiguos
   */
  @Cron(CronExpression.EVERY_WEEK)
  async handleWeeklyCleanup() {
    const executionId = this.jobTracker.generateExecutionId();
    const jobName = 'weekly-cleanup';

    const lockAcquired = await this.jobTracker.acquireJobLock(jobName, 7200);
    if (!lockAcquired) {
      return;
    }

    try {
      await this.jobTracker.startJob(jobName, executionId);

      this.logger.log('Iniciando limpieza semanal...');

      // Limpiar eventos de blockchain antiguos (más de 90 días)
      const ninetyDaysAgo = new Date();
      ninetyDaysAgo.setDate(ninetyDaysAgo.getDate() - 90);

      // Aquí se pueden agregar más limpiezas según sea necesario

      await this.jobTracker.saveJobState(jobName, {
        lastCleanup: new Date(),
      });

      this.logger.log('Limpieza semanal completada');

      await this.jobTracker.completeJob(jobName, executionId);
    } catch (error) {
      this.logger.error(
        `Error en limpieza semanal: ${error.message}`,
        error.stack,
      );
      await this.jobTracker.failJob(jobName, executionId, error.message);
    } finally {
      await this.jobTracker.releaseJobLock(jobName);
    }
  }
}