import {
  WebSocketGateway,
  WebSocketServer,
  SubscribeMessage,
  OnGatewayConnection,
  OnGatewayDisconnect,
  MessageBody,
  ConnectedSocket,
} from '@nestjs/websockets';
import { Server, Socket } from 'socket.io';
import { Logger, UseGuards } from '@nestjs/common';
import { JwtService } from '@nestjs/jwt';
import { ConfigService } from '@nestjs/config';
import { Notification, NotificationType } from '../../database/entities/notification.entity';

/**
 * WebSocket Gateway para Notificaciones
 * 
 * Rol: WebSocket
 * 
 * Contiene:
 * - Emisión de eventos al frontend
 * - Gestión de conexiones WebSocket
 * - Suscripciones a canales de notificaciones
 * - Autenticación de usuarios
 * 
 * ⚠️ REGLA FINAL: Este gateway NO contiene lógica crítica.
 * Solo emite eventos de notificaciones al frontend.
 * Si falla, no afecta la funcionalidad principal del sistema.
 */
@WebSocketGateway({
  cors: {
    origin: process.env.CORS_ORIGIN || '*',
    credentials: true,
  },
  namespace: '/notifications',
  transports: ['websocket', 'polling'],
})
export class NotificationsGateway
  implements OnGatewayConnection, OnGatewayDisconnect
{
  @WebSocketServer()
  server: Server;

  private readonly logger = new Logger(NotificationsGateway.name);
  private userSockets: Map<string, Set<string>> = new Map(); // userId -> Set of socketIds
  private socketUsers: Map<string, string> = new Map(); // socketId -> userId

  constructor(
    private readonly jwtService: JwtService,
    private readonly configService: ConfigService,
  ) {}

  /**
   * Maneja nuevas conexiones WebSocket
   */
  async handleConnection(client: Socket) {
    try {
      this.logger.debug(`Nueva conexión WebSocket: ${client.id}`);

      // Autenticación opcional por token
      const token =
        client.handshake.auth?.token || client.handshake.query?.token;

      if (token) {
        try {
          const payload = this.jwtService.verify(token as string, {
            secret: this.configService.get<string>('jwt.secret'),
          });

          // Asociar usuario con socket
          client.data.userId = payload.sub;
          client.data.walletAddress = payload.walletAddress;

          // Registrar socket del usuario
          if (!this.userSockets.has(payload.sub)) {
            this.userSockets.set(payload.sub, new Set());
          }
          this.userSockets.get(payload.sub)?.add(client.id);
          this.socketUsers.set(client.id, payload.sub);

          // Suscribir automáticamente a notificaciones del usuario
          const userChannel = `user:${payload.sub}`;
          client.join(userChannel);

          this.logger.log(
            `Cliente autenticado conectado: ${client.id} (user: ${payload.sub})`,
          );

          // Enviar confirmación de conexión
          client.emit('connected', {
            userId: payload.sub,
            walletAddress: payload.walletAddress,
            timestamp: new Date().toISOString(),
          });
        } catch (error) {
          this.logger.warn(`Token inválido para cliente ${client.id}`);
          client.emit('error', { message: 'Invalid authentication token' });
        }
      } else {
        this.logger.log(`Cliente anónimo conectado: ${client.id}`);
        client.emit('connected', {
          anonymous: true,
          timestamp: new Date().toISOString(),
        });
      }
    } catch (error) {
      this.logger.error(`Error en conexión: ${error.message}`, error.stack);
      client.emit('error', { message: 'Connection error' });
    }
  }

  /**
   * Maneja desconexiones WebSocket
   */
  handleDisconnect(client: Socket) {
    const userId = client.data?.userId;
    const socketId = client.id;

    if (userId) {
      const sockets = this.userSockets.get(userId);
      if (sockets) {
        sockets.delete(socketId);
        if (sockets.size === 0) {
          this.userSockets.delete(userId);
        }
      }
    }

    this.socketUsers.delete(socketId);

    this.logger.log(`Cliente desconectado: ${socketId} (user: ${userId || 'anonymous'})`);
  }

  /**
   * Suscribirse a notificaciones de un canal específico
   */
  @SubscribeMessage('subscribe')
  handleSubscribe(
    @ConnectedSocket() client: Socket,
    @MessageBody() data: { channel: string },
  ) {
    const { channel } = data;

    if (!channel) {
      return { error: 'Channel is required' };
    }

    // Validar que el canal sea válido
    const validChannels = [
      'notifications',
      'orders',
      'disputes',
      'escrow',
      'market',
      'price',
    ];

    if (!validChannels.includes(channel) && !channel.startsWith('user:')) {
      return { error: 'Invalid channel' };
    }

    client.join(channel);
    this.logger.debug(`Cliente ${client.id} se suscribió a: ${channel}`);

    return {
      event: 'subscribed',
      channel,
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Desuscribirse de un canal
   */
  @SubscribeMessage('unsubscribe')
  handleUnsubscribe(
    @ConnectedSocket() client: Socket,
    @MessageBody() data: { channel: string },
  ) {
    const { channel } = data;

    if (!channel) {
      return { error: 'Channel is required' };
    }

    client.leave(channel);
    this.logger.debug(`Cliente ${client.id} se desuscribió de: ${channel}`);

    return {
      event: 'unsubscribed',
      channel,
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Suscribirse a notificaciones del usuario autenticado
   */
  @SubscribeMessage('subscribe:user')
  handleSubscribeUser(@ConnectedSocket() client: Socket) {
    const userId = client.data?.userId;

    if (!userId) {
      return { error: 'Authentication required' };
    }

    const channel = `user:${userId}`;
    client.join(channel);

    this.logger.debug(
      `Cliente ${client.id} suscrito a notificaciones de usuario ${userId}`,
    );

    return {
      event: 'subscribed',
      channel,
      userId,
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Marcar notificación como leída desde el frontend
   */
  @SubscribeMessage('notification:read')
  handleNotificationRead(
    @ConnectedSocket() client: Socket,
    @MessageBody() data: { notificationId: string },
  ) {
    const userId = client.data?.userId;

    if (!userId) {
      return { error: 'Authentication required' };
    }

    const { notificationId } = data;

    if (!notificationId) {
      return { error: 'Notification ID is required' };
    }

    // Confirmar que se recibió la marca como leída
    // La lógica real está en NotificationsService
    this.logger.debug(
      `Usuario ${userId} marcó notificación ${notificationId} como leída`,
    );

    return {
      event: 'notification:read',
      notificationId,
      timestamp: new Date().toISOString(),
    };
  }

  // ============================================
  // MÉTODOS PARA EMITIR EVENTOS AL FRONTEND
  // ============================================

  /**
   * Emite una notificación a un usuario específico
   * 
   * Se conecta con: NotificationsService
   */
  emitNotification(userId: string, notification: Notification): void {
    try {
      const channel = `user:${userId}`;
      this.server.to(channel).emit('notification', {
        type: 'notification',
        notification,
        timestamp: new Date().toISOString(),
      });

      this.logger.debug(`Notification emitted to user: ${userId}`);
    } catch (error) {
      this.logger.error(
        `Error emitting notification to user ${userId}: ${error.message}`,
      );
    }
  }

  /**
   * Emite notificación de nueva orden
   */
  emitOrderNotification(
    userId: string,
    orderId: string,
    type: NotificationType,
    data: any,
  ): void {
    try {
      const channel = `user:${userId}`;
      this.server.to(channel).emit('notification:order', {
        type: 'order',
        orderId,
        notificationType: type,
        data,
        timestamp: new Date().toISOString(),
      });

      // También emitir al canal de órdenes
      this.server.to(`orders:${orderId}`).emit('order:update', {
        orderId,
        ...data,
        timestamp: new Date().toISOString(),
      });

      this.logger.debug(`Order notification emitted: ${orderId} to user ${userId}`);
    } catch (error) {
      this.logger.error(
        `Error emitting order notification: ${error.message}`,
      );
    }
  }

  /**
   * Emite notificación de disputa
   */
  emitDisputeNotification(
    userId: string,
    disputeId: string,
    type: NotificationType,
    data: any,
  ): void {
    try {
      const channel = `user:${userId}`;
      this.server.to(channel).emit('notification:dispute', {
        type: 'dispute',
        disputeId,
        notificationType: type,
        data,
        timestamp: new Date().toISOString(),
      });

      // También emitir al canal de disputas
      this.server.to(`disputes:${disputeId}`).emit('dispute:update', {
        disputeId,
        ...data,
        timestamp: new Date().toISOString(),
      });

      this.logger.debug(
        `Dispute notification emitted: ${disputeId} to user ${userId}`,
      );
    } catch (error) {
      this.logger.error(
        `Error emitting dispute notification: ${error.message}`,
      );
    }
  }

  /**
   * Emite notificación de escrow
   */
  emitEscrowNotification(
    userId: string,
    escrowId: string,
    type: NotificationType,
    data: any,
  ): void {
    try {
      const channel = `user:${userId}`;
      this.server.to(channel).emit('notification:escrow', {
        type: 'escrow',
        escrowId,
        notificationType: type,
        data,
        timestamp: new Date().toISOString(),
      });

      // También emitir al canal de escrow
      this.server.to(`escrow:${escrowId}`).emit('escrow:update', {
        escrowId,
        ...data,
        timestamp: new Date().toISOString(),
      });

      this.logger.debug(
        `Escrow notification emitted: ${escrowId} to user ${userId}`,
      );
    } catch (error) {
      this.logger.error(
        `Error emitting escrow notification: ${error.message}`,
      );
    }
  }

  /**
   * Emite notificación de cambio de reputación
   */
  emitReputationNotification(
    userId: string,
    newScore: number,
    change: number,
  ): void {
    try {
      const channel = `user:${userId}`;
      this.server.to(channel).emit('notification:reputation', {
        type: 'reputation',
        newScore,
        change,
        timestamp: new Date().toISOString(),
      });

      this.logger.debug(
        `Reputation notification emitted to user ${userId}: ${change > 0 ? '+' : ''}${change}`,
      );
    } catch (error) {
      this.logger.error(
        `Error emitting reputation notification: ${error.message}`,
      );
    }
  }

  /**
   * Emite actualización de mercado (broadcast)
   */
  emitMarketUpdate(data: any): void {
    try {
      this.server.to('market').emit('market:update', {
        ...data,
        timestamp: new Date().toISOString(),
      });

      this.logger.debug('Market update emitted');
    } catch (error) {
      this.logger.error(`Error emitting market update: ${error.message}`);
    }
  }

  /**
   * Emite actualización de precio (broadcast)
   */
  emitPriceUpdate(symbol: string, price: number, change?: number): void {
    try {
      this.server.to('price').emit('price:update', {
        symbol,
        price,
        change,
        timestamp: new Date().toISOString(),
      });

      this.logger.debug(`Price update emitted: ${symbol} = ${price}`);
    } catch (error) {
      this.logger.error(`Error emitting price update: ${error.message}`);
    }
  }

  /**
   * Emite conteo de notificaciones no leídas
   */
  emitUnreadCount(userId: string, count: number): void {
    try {
      const channel = `user:${userId}`;
      this.server.to(channel).emit('notifications:unread-count', {
        count,
        timestamp: new Date().toISOString(),
      });

      this.logger.debug(`Unread count emitted to user ${userId}: ${count}`);
    } catch (error) {
      this.logger.error(`Error emitting unread count: ${error.message}`);
    }
  }

  /**
   * Emite evento genérico de estado
   */
  emitStatusChange(
    entity: string,
    entityId: string,
    status: string,
    data?: any,
  ): void {
    try {
      this.server.to(`${entity}:${entityId}`).emit('status:change', {
        entity,
        entityId,
        status,
        data,
        timestamp: new Date().toISOString(),
      });

      this.logger.debug(
        `Status change emitted: ${entity}:${entityId} -> ${status}`,
      );
    } catch (error) {
      this.logger.error(`Error emitting status change: ${error.message}`);
    }
  }

  /**
   * Obtiene información sobre conexiones activas
   */
  getConnectionInfo(): {
    totalConnections: number;
    authenticatedUsers: number;
    channels: string[];
  } {
    const channels = Array.from(this.server.sockets.adapter.rooms.keys());
    const authenticatedUsers = this.userSockets.size;
    const totalConnections = this.server.sockets.sockets.size;

    return {
      totalConnections,
      authenticatedUsers,
      channels: channels.filter((ch) => !ch.startsWith('socket_')),
    };
  }

  /**
   * Verifica si un usuario está conectado
   */
  isUserConnected(userId: string): boolean {
    return this.userSockets.has(userId) && this.userSockets.get(userId)!.size > 0;
  }

  /**
   * Obtiene el número de conexiones de un usuario
   */
  getUserConnectionCount(userId: string): number {
    return this.userSockets.get(userId)?.size || 0;
  }
}

