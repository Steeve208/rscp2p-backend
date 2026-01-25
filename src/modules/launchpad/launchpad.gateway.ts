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
import { Logger } from '@nestjs/common';
import { JwtService } from '@nestjs/jwt';
import { ConfigService } from '@nestjs/config';

@WebSocketGateway({
  cors: {
    origin: '*',
    credentials: true,
  },
  namespace: '/launchpad',
  transports: ['websocket', 'polling'],
})
export class LaunchpadGateway implements OnGatewayConnection, OnGatewayDisconnect {
  @WebSocketServer()
  server: Server;

  private readonly logger = new Logger(LaunchpadGateway.name);
  private userSockets: Map<string, Set<string>> = new Map();

  constructor(
    private readonly jwtService: JwtService,
    private readonly configService: ConfigService,
  ) {}

  async handleConnection(client: Socket) {
    try {
      const token = client.handshake.auth?.token || client.handshake.query?.token;
      if (token) {
        try {
          const payload = this.jwtService.verify(token as string, {
            secret: this.configService.get<string>('jwt.secret'),
          });
          client.data.userId = payload.sub;
          client.data.walletAddress = payload.walletAddress;

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

  @SubscribeMessage('presale:subscribe')
  handlePresaleSubscribe(
    @ConnectedSocket() client: Socket,
    @MessageBody() data: { presaleId?: string },
  ) {
    const channel = data?.presaleId ? `presale:${data.presaleId}` : 'presale:all';
    client.join(channel);
    this.logger.debug(`Cliente ${client.id} suscrito a: ${channel}`);
    return { event: 'subscribed', channel, timestamp: new Date().toISOString() };
  }

  @SubscribeMessage('presale:unsubscribe')
  handlePresaleUnsubscribe(
    @ConnectedSocket() client: Socket,
    @MessageBody() data: { presaleId?: string },
  ) {
    const channel = data?.presaleId ? `presale:${data.presaleId}` : 'presale:all';
    client.leave(channel);
    this.logger.debug(`Cliente ${client.id} desuscrito de: ${channel}`);
    return { event: 'unsubscribed', channel, timestamp: new Date().toISOString() };
  }

  emitPresaleContribution(presaleId: string, payload: any) {
    try {
      this.server.to(`presale:${presaleId}`).emit('presale:contribution', payload);
      this.server.to('presale:all').emit('presale:contribution', payload);
      this.logger.debug(`Presale contribution emitted: ${presaleId}`);
    } catch (error) {
      this.logger.error(`Error emitting presale contribution: ${error.message}`);
    }
  }
}
