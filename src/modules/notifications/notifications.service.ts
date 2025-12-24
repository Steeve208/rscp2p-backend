import { Injectable, Logger, Inject, Optional } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { Notification, NotificationType } from '../../database/entities/notification.entity';
import { NotificationsGateway } from './notifications.gateway';

@Injectable()
export class NotificationsService {
  private readonly logger = new Logger(NotificationsService.name);

  constructor(
    @InjectRepository(Notification)
    private readonly notificationRepository: Repository<Notification>,
    @Optional()
    @Inject(NotificationsGateway)
    private readonly notificationsGateway?: NotificationsGateway,
  ) {}

  /**
   * Crea una notificación y la envía por WebSocket
   * NO contiene lógica crítica, solo notificaciones
   */
  async create(
    userId: string,
    type: NotificationType,
    title: string,
    message: string,
    data?: any,
  ): Promise<Notification> {
    const notification = this.notificationRepository.create({
      userId,
      type,
      title,
      message,
      data,
      read: false,
      orderId: data?.orderId,
      disputeId: data?.disputeId,
      escrowId: data?.escrowId,
    });

    const savedNotification = await this.notificationRepository.save(notification);

    // Enviar por WebSocket (no crítico si falla)
    if (this.notificationsGateway) {
      try {
        this.notificationsGateway.emitNotification(userId, savedNotification);
        
        // Emitir conteo de no leídas
        const unreadCount = await this.getUnreadCount(userId);
        this.notificationsGateway.emitUnreadCount(userId, unreadCount);
      } catch (error) {
        this.logger.warn(`Failed to send WebSocket notification: ${error.message}`);
      }
    }

    this.logger.debug(`Notification created: ${type} for user ${userId}`);

    return savedNotification;
  }

  /**
   * Notifica cambio de estado de orden
   */
  async notifyOrderStatusChange(
    userId: string,
    orderId: string,
    status: string,
    data?: any,
  ): Promise<void> {
    const typeMap: Record<string, NotificationType> = {
      CREATED: NotificationType.ORDER_CREATED,
      AWAITING_FUNDS: NotificationType.ORDER_ACCEPTED,
      COMPLETED: NotificationType.ORDER_COMPLETED,
      REFUNDED: NotificationType.ORDER_CANCELLED,
      DISPUTED: NotificationType.ORDER_DISPUTED,
    };

    const type = typeMap[status] || NotificationType.ORDER_CREATED;
    const title = `Orden ${orderId.substring(0, 8)}...`;
    const message = this.getOrderStatusMessage(status);

    const notification = await this.create(userId, type, title, message, {
      orderId,
      status,
      ...data,
    });

    // Emitir notificación de orden por WebSocket
    if (this.notificationsGateway) {
      try {
        this.notificationsGateway.emitOrderNotification(
          userId,
          orderId,
          type,
          { status, ...data },
        );
      } catch (error) {
        this.logger.warn(`Failed to emit order notification: ${error.message}`);
      }
    }
  }

  /**
   * Notifica apertura de disputa
   */
  async notifyDisputeOpened(
    userId: string,
    disputeId: string,
    orderId: string,
  ): Promise<void> {
    const notification = await this.create(
      userId,
      NotificationType.DISPUTE_OPENED,
      'Nueva disputa abierta',
      `Se ha abierto una disputa para la orden ${orderId.substring(0, 8)}...`,
      { disputeId, orderId },
    );

    // Emitir notificación de disputa por WebSocket
    if (this.notificationsGateway) {
      try {
        this.notificationsGateway.emitDisputeNotification(
          userId,
          disputeId,
          NotificationType.DISPUTE_OPENED,
          { orderId },
        );
      } catch (error) {
        this.logger.warn(`Failed to emit dispute notification: ${error.message}`);
      }
    }
  }

  /**
   * Notifica resolución de disputa
   */
  async notifyDisputeResolved(
    userId: string,
    disputeId: string,
    orderId: string,
    resolution: string,
  ): Promise<void> {
    const notification = await this.create(
      userId,
      NotificationType.DISPUTE_RESOLVED,
      'Disputa resuelta',
      `La disputa para la orden ${orderId.substring(0, 8)}... ha sido resuelta: ${resolution}`,
      { disputeId, orderId, resolution },
    );

    // Emitir notificación de disputa resuelta por WebSocket
    if (this.notificationsGateway) {
      try {
        this.notificationsGateway.emitDisputeNotification(
          userId,
          disputeId,
          NotificationType.DISPUTE_RESOLVED,
          { orderId, resolution },
        );
      } catch (error) {
        this.logger.warn(`Failed to emit dispute notification: ${error.message}`);
      }
    }
  }

  /**
   * Notifica cambio de estado de escrow
   */
  async notifyEscrowStatusChange(
    userId: string,
    escrowId: string,
    status: string,
    data?: any,
  ): Promise<void> {
    const typeMap: Record<string, NotificationType> = {
      LOCKED: NotificationType.ESCROW_LOCKED,
      RELEASED: NotificationType.ESCROW_RELEASED,
      REFUNDED: NotificationType.ESCROW_REFUNDED,
    };

    const type = typeMap[status] || NotificationType.ESCROW_LOCKED;
    const title = `Escrow ${escrowId.substring(0, 8)}...`;
    const message = this.getEscrowStatusMessage(status);

    const notification = await this.create(userId, type, title, message, {
      escrowId,
      status,
      ...data,
    });

    // Emitir notificación de escrow por WebSocket
    if (this.notificationsGateway) {
      try {
        this.notificationsGateway.emitEscrowNotification(
          userId,
          escrowId,
          type,
          { status, ...data },
        );
      } catch (error) {
        this.logger.warn(`Failed to emit escrow notification: ${error.message}`);
      }
    }
  }

  /**
   * Notifica actualización de mercado
   */
  async notifyMarketUpdate(
    userId: string,
    update: any,
  ): Promise<void> {
    await this.create(
      userId,
      NotificationType.MARKET_UPDATE,
      'Actualización de mercado',
      update.message || 'Nueva actualización disponible en el mercado',
      update,
    );
  }

  /**
   * Notifica actualización de precio
   */
  async notifyPriceUpdate(
    symbol: string,
    price: number,
    change: number,
  ): Promise<void> {
    // Emitir a todos los usuarios suscritos (no crítico)
    if (this.notificationsGateway) {
      try {
        this.notificationsGateway.emitPriceUpdate(symbol, price, change);
      } catch (error) {
        this.logger.warn(`Failed to emit price update: ${error.message}`);
      }
    }
  }

  /**
   * Notifica cambio de reputation
   */
  async notifyReputationChange(
    userId: string,
    newScore: number,
    change: number,
  ): Promise<void> {
    const notification = await this.create(
      userId,
      NotificationType.REPUTATION_CHANGE,
      'Cambio de reputación',
      `Tu reputación ha cambiado: ${change > 0 ? '+' : ''}${change} (${newScore})`,
      { newScore, change },
    );

    // Emitir notificación de reputación por WebSocket
    if (this.notificationsGateway) {
      try {
        this.notificationsGateway.emitReputationNotification(userId, newScore, change);
      } catch (error) {
        this.logger.warn(`Failed to emit reputation notification: ${error.message}`);
      }
    }
  }

  /**
   * Obtiene notificaciones de un usuario
   */
  async getUserNotifications(
    userId: string,
    limit: number = 50,
    unreadOnly: boolean = false,
  ): Promise<Notification[]> {
    const where: any = { userId };
    if (unreadOnly) {
      where.read = false;
    }

    return this.notificationRepository.find({
      where,
      order: { createdAt: 'DESC' },
      take: limit,
    });
  }

  /**
   * Marca notificación como leída
   */
  async markAsRead(notificationId: string, userId: string): Promise<Notification> {
    const notification = await this.notificationRepository.findOne({
      where: { id: notificationId, userId },
    });

    if (!notification) {
      throw new Error('Notification not found');
    }

    notification.read = true;
    notification.readAt = new Date();
    return this.notificationRepository.save(notification);
  }

  /**
   * Marca todas las notificaciones como leídas
   */
  async markAllAsRead(userId: string): Promise<void> {
    await this.notificationRepository.update(
      { userId, read: false },
      { read: true, readAt: new Date() },
    );
  }

  /**
   * Obtiene conteo de notificaciones no leídas
   */
  async getUnreadCount(userId: string): Promise<number> {
    return this.notificationRepository.count({
      where: { userId, read: false },
    });
  }

  /**
   * Elimina notificaciones antiguas
   */
  async deleteOldNotifications(days: number = 30): Promise<number> {
    const date = new Date();
    date.setDate(date.getDate() - days);

    const result = await this.notificationRepository
      .createQueryBuilder()
      .delete()
      .where('created_at < :date', { date })
      .andWhere('read = :read', { read: true })
      .execute();

    return result.affected || 0;
  }

  /**
   * Mensajes de estado de orden
   */
  private getOrderStatusMessage(status: string): string {
    const messages: Record<string, string> = {
      CREATED: 'Tu orden ha sido creada',
      AWAITING_FUNDS: 'Tu orden ha sido aceptada, esperando fondos',
      ONCHAIN_LOCKED: 'Fondos bloqueados en escrow',
      COMPLETED: 'Tu orden ha sido completada',
      REFUNDED: 'Tu orden ha sido cancelada',
      DISPUTED: 'Se ha abierto una disputa para tu orden',
    };
    return messages[status] || 'Estado de orden actualizado';
  }

  /**
   * Mensajes de estado de escrow
   */
  private getEscrowStatusMessage(status: string): string {
    const messages: Record<string, string> = {
      LOCKED: 'Fondos bloqueados en escrow',
      RELEASED: 'Fondos liberados del escrow',
      REFUNDED: 'Fondos reembolsados del escrow',
    };
    return messages[status] || 'Estado de escrow actualizado';
  }
}