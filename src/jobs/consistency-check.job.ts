import { Injectable, Logger, Inject } from '@nestjs/common';
import { Cron, CronExpression } from '@nestjs/schedule';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { Order } from '../database/entities/order.entity';
import { Escrow } from '../database/entities/escrow.entity';
import { Dispute } from '../database/entities/dispute.entity';
import { OrderStatus } from '../common/enums/order-status.enum';
import { EscrowStatus } from '../common/enums/escrow-status.enum';
import { DisputeStatus } from '../common/enums/dispute-status.enum';
import { JobTrackerService } from './job-tracker.service';
import { EscrowService } from '../modules/escrow/escrow.service';

@Injectable()
export class ConsistencyCheckJob {
  private readonly logger = new Logger(ConsistencyCheckJob.name);

  constructor(
    @InjectRepository(Order)
    private readonly orderRepository: Repository<Order>,
    @InjectRepository(Escrow)
    private readonly escrowRepository: Repository<Escrow>,
    @InjectRepository(Dispute)
    private readonly disputeRepository: Repository<Dispute>,
    private readonly jobTracker: JobTrackerService,
    private readonly escrowService: EscrowService,
  ) {}

  /**
   * Verificación de inconsistencias cada 30 minutos
   * Crítico: detecta y reporta problemas
   */
  @Cron('0 */30 * * * *') // Cada 30 minutos
  async handleConsistencyCheck() {
    const executionId = this.jobTracker.generateExecutionId();
    const jobName = 'consistency-check';

    const lockAcquired = await this.jobTracker.acquireJobLock(jobName, 1800);
    if (!lockAcquired) {
      this.logger.debug('Consistency check already running');
      return;
    }

    try {
      await this.jobTracker.startJob(jobName, executionId);

      this.logger.log('Iniciando verificación de inconsistencias...');

      const issues: string[] = [];

      // 1. Verificar consistencia entre órdenes y escrows
      const ordersWithEscrow = await this.orderRepository.find({
        where: [
          { status: OrderStatus.ONCHAIN_LOCKED },
          { status: OrderStatus.COMPLETED },
          { status: OrderStatus.REFUNDED },
        ],
      });

      for (const order of ordersWithEscrow) {
        if (order.escrowId) {
          try {
            const validation = await this.escrowService.validateConsistency(order.id);
            if (!validation.isValid) {
              issues.push(
                `Order ${order.id}: ${validation.errors.join(', ')}`,
              );
            }
          } catch (error) {
            issues.push(`Order ${order.id}: Error validating - ${error.message}`);
          }
        } else if (
          order.status === OrderStatus.ONCHAIN_LOCKED ||
          order.status === OrderStatus.COMPLETED
        ) {
          issues.push(
            `Order ${order.id}: Status ${order.status} but no escrowId`,
          );
        }
      }

      // 2. Verificar escrows sin orden
      const escrows = await this.escrowRepository.find();
      for (const escrow of escrows) {
        const order = await this.orderRepository.findOne({
          where: { id: escrow.orderId },
        });
        if (!order) {
          issues.push(`Escrow ${escrow.escrowId}: Order ${escrow.orderId} not found`);
        }
      }

      // 3. Verificar estados inconsistentes
      const inconsistentOrders = await this.orderRepository
        .createQueryBuilder('order')
        .leftJoinAndSelect('order.escrowId', 'escrow')
        .where('order.status = :status1', { status1: OrderStatus.ONCHAIN_LOCKED })
        .andWhere('escrow.status != :escrowStatus', {
          escrowStatus: EscrowStatus.LOCKED,
        })
        .getMany();

      for (const order of inconsistentOrders) {
        issues.push(
          `Order ${order.id}: Status ONCHAIN_LOCKED but escrow not LOCKED`,
        );
      }

      // 4. Verificar disputas sin orden
      const disputes = await this.disputeRepository.find();
      for (const dispute of disputes) {
        const order = await this.orderRepository.findOne({
          where: { id: dispute.orderId },
        });
        if (!order) {
          issues.push(`Dispute ${dispute.id}: Order ${dispute.orderId} not found`);
        } else if (order.status !== OrderStatus.DISPUTED && dispute.status === DisputeStatus.OPEN) {
          issues.push(
            `Dispute ${dispute.id}: Open but order status is ${order.status}`,
          );
        }
      }

      // Guardar estado y resultados
      const result = {
        checkedAt: new Date(),
        issuesFound: issues.length,
        issues,
        ordersChecked: ordersWithEscrow.length,
        escrowsChecked: escrows.length,
        disputesChecked: disputes.length,
      };

      await this.jobTracker.saveJobState(jobName, result);

      if (issues.length > 0) {
        this.logger.warn(
          `Inconsistencias detectadas: ${issues.length}`,
        );
        issues.forEach((issue) => this.logger.warn(`  - ${issue}`));
      } else {
        this.logger.log('No se encontraron inconsistencias');
      }

      await this.jobTracker.completeJob(jobName, executionId, result);
    } catch (error) {
      this.logger.error(
        `Error en verificación de inconsistencias: ${error.message}`,
        error.stack,
      );
      await this.jobTracker.failJob(jobName, executionId, error.message);
    } finally {
      await this.jobTracker.releaseJobLock(jobName);
    }
  }

  /**
   * Verificación profunda semanal
   */
  @Cron(CronExpression.EVERY_WEEK)
  async handleDeepConsistencyCheck() {
    const executionId = this.jobTracker.generateExecutionId();
    const jobName = 'deep-consistency-check';

    const lockAcquired = await this.jobTracker.acquireJobLock(jobName, 7200);
    if (!lockAcquired) {
      return;
    }

    try {
      await this.jobTracker.startJob(jobName, executionId);

      this.logger.log('Iniciando verificación profunda de inconsistencias...');

      // Ejecutar todas las verificaciones normales
      await this.handleConsistencyCheck();

      // Verificaciones adicionales profundas
      // (pueden agregarse más según necesidad)

      await this.jobTracker.completeJob(jobName, executionId);
    } catch (error) {
      this.logger.error(
        `Error en verificación profunda: ${error.message}`,
        error.stack,
      );
      await this.jobTracker.failJob(jobName, executionId, error.message);
    } finally {
      await this.jobTracker.releaseJobLock(jobName);
    }
  }
}
