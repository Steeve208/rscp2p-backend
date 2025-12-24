import { Injectable, Logger } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { Escrow } from '../../../database/entities/escrow.entity';
import { Order } from '../../../database/entities/order.entity';
import { BlockchainEvent } from '../../../database/entities/blockchain-event.entity';
import { EscrowStatus } from '../../../common/enums/escrow-status.enum';
import { OrderStatus } from '../../../common/enums/order-status.enum';
import { EscrowService } from '../../escrow/escrow.service';

@Injectable()
export class StateReconcilerService {
  private readonly logger = new Logger(StateReconcilerService.name);

  constructor(
    @InjectRepository(Escrow)
    private readonly escrowRepository: Repository<Escrow>,
    @InjectRepository(Order)
    private readonly orderRepository: Repository<Order>,
    @InjectRepository(BlockchainEvent)
    private readonly eventRepository: Repository<BlockchainEvent>,
    private readonly escrowService: EscrowService,
  ) {}

  /**
   * Reconcilia el estado de un escrow con la blockchain
   */
  async reconcileEscrow(escrowId: string): Promise<{
    reconciled: boolean;
    changes: string[];
  }> {
    const changes: string[] = [];

    try {
      const escrow = await this.escrowRepository.findOne({
        where: { escrowId },
      });

      if (!escrow) {
        this.logger.warn(`Escrow not found: ${escrowId}`);
        return { reconciled: false, changes: ['Escrow not found'] };
      }

      // Obtener eventos relacionados con este escrow
      const events = await this.eventRepository.find({
        where: { escrowId, processed: false },
        order: { blockNumber: 'ASC' },
      });

      for (const event of events) {
        const change = await this.processEvent(event, escrow);
        if (change) {
          changes.push(change);
        }
      }

      // Validar consistencia después de reconciliar
      const validation = await this.escrowService.validateConsistency(
        escrow.orderId,
      );

      if (!validation.isValid) {
        changes.push(`Validation errors: ${validation.errors.join(', ')}`);
      }

      this.logger.log(
        `Reconciled escrow ${escrowId}: ${changes.length} changes`,
      );

      return { reconciled: true, changes };
    } catch (error) {
      this.logger.error(
        `Error reconciling escrow ${escrowId}: ${error.message}`,
        error.stack,
      );
      return { reconciled: false, changes: [error.message] };
    }
  }

  /**
   * Procesa un evento y actualiza el estado
   */
  private async processEvent(
    event: BlockchainEvent,
    escrow: Escrow,
  ): Promise<string | null> {
    try {
      let change: string | null = null;

      switch (event.eventName) {
        case 'EscrowCreated':
          // El escrow ya debería existir, solo marcar como procesado
          break;

        case 'FundsLocked':
          if (escrow.status !== EscrowStatus.LOCKED) {
            await this.escrowService.update(escrow.escrowId, {
              status: EscrowStatus.LOCKED,
            });
            change = `Escrow ${escrow.escrowId} locked`;
          }
          break;

        case 'FundsReleased':
          if (escrow.status !== EscrowStatus.RELEASED) {
            await this.escrowService.update(escrow.escrowId, {
              status: EscrowStatus.RELEASED,
              releaseTransactionHash: event.transactionHash,
            });
            // Actualizar orden a completada
            const order = await this.orderRepository.findOne({
              where: { id: escrow.orderId },
            });
            if (order && order.status !== OrderStatus.COMPLETED) {
              // Usar el servicio de orders si está disponible, sino actualizar directamente
              order.status = OrderStatus.COMPLETED;
              order.completedAt = new Date();
              await this.orderRepository.save(order);
            }
            change = `Escrow ${escrow.escrowId} released`;
          }
          break;

        case 'FundsRefunded':
          if (escrow.status !== EscrowStatus.REFUNDED) {
            await this.escrowService.update(escrow.escrowId, {
              status: EscrowStatus.REFUNDED,
              refundTransactionHash: event.transactionHash,
            });
            // Actualizar orden a reembolsada
            const order = await this.orderRepository.findOne({
              where: { id: escrow.orderId },
            });
            if (order && order.status !== OrderStatus.REFUNDED) {
              order.status = OrderStatus.REFUNDED;
              order.cancelledAt = new Date();
              await this.orderRepository.save(order);
            }
            change = `Escrow ${escrow.escrowId} refunded`;
          }
          break;

        case 'DisputeOpened':
          if (escrow.status !== EscrowStatus.DISPUTED) {
            await this.escrowService.update(escrow.escrowId, {
              status: EscrowStatus.DISPUTED,
            });
            // Actualizar orden a disputada
            const order = await this.orderRepository.findOne({
              where: { id: escrow.orderId },
            });
            if (order && order.status !== OrderStatus.DISPUTED) {
              order.status = OrderStatus.DISPUTED;
              await this.orderRepository.save(order);
            }
            change = `Escrow ${escrow.escrowId} disputed`;
          }
          break;
      }

      // Marcar evento como procesado
      event.processed = true;
      event.processedAt = new Date();
      await this.eventRepository.save(event);

      return change;
    } catch (error) {
      this.logger.error(
        `Error processing event ${event.id}: ${error.message}`,
        error.stack,
      );
      event.errorMessage = error.message;
      await this.eventRepository.save(event);
      return null;
    }
  }

  /**
   * Reconcilia todos los escrows pendientes
   */
  async reconcileAll(): Promise<{
    total: number;
    reconciled: number;
    errors: number;
  }> {
    const escrows = await this.escrowRepository.find({
      where: [
        { status: EscrowStatus.PENDING },
        { status: EscrowStatus.LOCKED },
      ],
    });

    let reconciled = 0;
    let errors = 0;

    for (const escrow of escrows) {
      const result = await this.reconcileEscrow(escrow.escrowId);
      if (result.reconciled) {
        reconciled++;
      } else {
        errors++;
      }
    }

    this.logger.log(
      `Reconciled ${reconciled}/${escrows.length} escrows. Errors: ${errors}`,
    );

    return {
      total: escrows.length,
      reconciled,
      errors,
    };
  }

  /**
   * Reconcilia eventos no procesados
   */
  async reconcileUnprocessedEvents(): Promise<{
    total: number;
    processed: number;
    errors: number;
  }> {
    const events = await this.eventRepository.find({
      where: { processed: false },
      order: { blockNumber: 'ASC' },
    });

    let processed = 0;
    let errors = 0;

    for (const event of events) {
      if (event.escrowId) {
        const escrow = await this.escrowRepository.findOne({
          where: { escrowId: event.escrowId },
        });

        if (escrow) {
          const change = await this.processEvent(event, escrow);
          if (change) {
            processed++;
          } else {
            errors++;
          }
        } else {
          this.logger.warn(
            `Event ${event.id} references unknown escrow ${event.escrowId}`,
          );
          errors++;
        }
      } else {
        this.logger.warn(`Event ${event.id} has no escrowId`);
        errors++;
      }
    }

    return {
      total: events.length,
      processed,
      errors,
    };
  }
}
