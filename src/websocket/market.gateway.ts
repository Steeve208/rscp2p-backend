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

@WebSocketGateway({
  cors: {
    origin: '*',
    credentials: true,
  },
  namespace: '/market',
})
export class MarketGateway implements OnGatewayConnection, OnGatewayDisconnect {
  @WebSocketServer()
  server: Server;

  private readonly logger = new Logger(MarketGateway.name);
  private userSockets: Map<string, Set<string>> = new Map(); // userId -> Set of socketIds

  constructor(
    private readonly jwtService: JwtService,
    private readonly configService: ConfigService,
  ) {}

  async handleConnection(client: Socket) {
    try {
      // Autenticación opcional por token
      const token = client.handshake.auth?.token || client.handshake.query?.token;
      
      if (token) {
        try {
          const payload = this.jwtService.verify(token as string, {
            secret: this.configService.get<string>('jwt.secret'),
          });
          client.data.userId = payload.sub;
          client.data.walletAddress = payload.walletAddress;

          // Registrar socket del usuario
          if (!this.userSockets.has(payload.sub)) {
            this.userSockets.set(payload.sub, new Set());
          }
          this.userSockets.get(payload.sub)?.add(client.id);

          this.logger.log(`Cliente autenticado conectado: ${client.id} (user: ${payload.sub})`);
        } catch (error) {
          this.logger.warn(`Token inválido para cliente ${client.id}`);
        }
      } else {
        this.logger.log(`Cliente anónimo conectado: ${client.id}`);
      }
    } catch (error) {
      this.logger.error(`Error en conexión: ${error.message}`);
    }
  }

  handleDisconnect(client: Socket) {
    const userId = client.data?.userId;
    if (userId) {
      const sockets = this.userSockets.get(userId);
      if (sockets) {
        sockets.delete(client.id);
        if (sockets.size === 0) {
          this.userSockets.delete(userId);
        }
      }
    }
    this.logger.log(`Cliente desconectado: ${client.id}`);
  }

  @SubscribeMessage('subscribe')
  handleSubscribe(@ConnectedSocket() client: Socket, @MessageBody() data: any) {
    const channel = data.channel;
    if (!channel) {
      return { error: 'Channel is required' };
    }

    this.logger.debug(`Cliente ${client.id} se suscribió a: ${channel}`);
    client.join(channel);
    return { event: 'subscribed', channel };
  }

  @SubscribeMessage('unsubscribe')
  handleUnsubscribe(@ConnectedSocket() client: Socket, @MessageBody() data: any) {
    const channel = data.channel;
    if (!channel) {
      return { error: 'Channel is required' };
    }

    this.logger.debug(`Cliente ${client.id} se desuscribió de: ${channel}`);
    client.leave(channel);
    return { event: 'unsubscribed', channel };
  }

  @SubscribeMessage('subscribe:user')
  handleSubscribeUser(@ConnectedSocket() client: Socket) {
    const userId = client.data?.userId;
    if (!userId) {
      return { error: 'Authentication required' };
    }

    const channel = `user:${userId}`;
    client.join(channel);
    this.logger.debug(`Cliente ${client.id} suscrito a notificaciones de usuario ${userId}`);
    return { event: 'subscribed', channel };
  }

  // Métodos para emitir eventos desde el servidor (no críticos)

  /**
   * Emite actualización de orden
   */
  emitOrderUpdate(orderId: string, data: any) {
    this.server.to(`order:${orderId}`).emit('order:update', data);
    this.logger.debug(`Order update emitted: ${orderId}`);
  }

  /**
   * Emite actualización de mercado
   */
  emitMarketUpdate(data: any) {
    this.server.to('market:updates').emit('market:update', data);
    this.logger.debug('Market update emitted');
  }

  /**
   * Emite actualización de precio
   */
  emitPriceUpdate(symbol: string, price: number) {
    this.server.to(`price:${symbol}`).emit('price:update', { symbol, price });
    this.logger.debug(`Price update emitted: ${symbol} = ${price}`);
  }

  /**
   * Emite notificación a un usuario específico
   */
  emitToUser(userId: string, data: any) {
    this.server.to(`user:${userId}`).emit('notification', data);
    this.logger.debug(`Notification emitted to user: ${userId}`);
  }

  /**
   * Emite evento de cambio de estado
   */
  emitStatusChange(entity: string, entityId: string, status: string, data?: any) {
    this.server.to(`${entity}:${entityId}`).emit('status:change', {
      entity,
      entityId,
      status,
      data,
    });
    this.logger.debug(`Status change emitted: ${entity}:${entityId} -> ${status}`);
  }

  /**
   * Emite evento de disputa
   */
  emitDisputeEvent(disputeId: string, event: string, data: any) {
    this.server.to(`dispute:${disputeId}`).emit(`dispute:${event}`, data);
    this.logger.debug(`Dispute event emitted: ${event} for ${disputeId}`);
  }

  /**
   * Emite evento de escrow
   */
  emitEscrowEvent(escrowId: string, event: string, data: any) {
    this.server.to(`escrow:${escrowId}`).emit(`escrow:${event}`, data);
    this.logger.debug(`Escrow event emitted: ${event} for ${escrowId}`);
  }
}