import {
  Injectable,
  UnauthorizedException,
  BadRequestException,
  Inject,
  Logger,
} from '@nestjs/common';
import { ConfigService } from '@nestjs/config';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { JwtService } from '@nestjs/jwt';
import { Redis } from 'ioredis';
import { ethers } from 'ethers';
import { User } from '../../database/entities/user.entity';
import { ChallengeDto, VerifyDto, RefreshDto } from './dto';

@Injectable()
export class AuthService {
  private readonly logger = new Logger(AuthService.name);
  private readonly nonceTtl = 300; // 5 minutos
  private readonly sessionTtl = 86400; // 24 horas
  private readonly refreshTokenTtl = 604800; // 7 días

  constructor(
    @InjectRepository(User)
    private readonly userRepository: Repository<User>,
    @Inject('REDIS_CLIENT')
    private readonly redis: Redis,
    private readonly jwtService: JwtService,
    private readonly configService: ConfigService,
  ) {}

  /**
   * Genera un challenge (nonce) para que el usuario lo firme con su wallet
   */
  async generateChallenge(dto: ChallengeDto): Promise<{ nonce: string; message: string }> {
    const { walletAddress } = dto;
    const normalizedAddress = ethers.getAddress(walletAddress);

    // Verificar rate limiting anti-spam
    await this.checkRateLimit(normalizedAddress, 'challenge');

    // Generar nonce único
    const nonce = this.generateNonce();
    const message = this.createSignMessage(normalizedAddress, nonce);

    // Guardar nonce en Redis con TTL
    const nonceKey = `auth:nonce:${normalizedAddress}:${nonce}`;
    await this.redis.setex(nonceKey, this.nonceTtl, JSON.stringify({
      walletAddress: normalizedAddress,
      nonce,
      createdAt: Date.now(),
    }));

    this.logger.debug(`Challenge generated for ${normalizedAddress}`);

    return { nonce, message };
  }

  /**
   * Verifica la firma del usuario y crea una sesión
   */
  async verifySignature(dto: VerifyDto): Promise<{
    accessToken: string;
    refreshToken: string;
    user: Partial<User>;
  }> {
    const { walletAddress, nonce, signature } = dto;
    const normalizedAddress = ethers.getAddress(walletAddress);

    // Verificar rate limiting
    await this.checkRateLimit(normalizedAddress, 'verify');

    // Verificar que el nonce existe y es válido
    const nonceKey = `auth:nonce:${normalizedAddress}:${nonce}`;
    const nonceData = await this.redis.get(nonceKey);

    if (!nonceData) {
      throw new BadRequestException('Nonce inválido o expirado');
    }

    // Verificar la firma
    const message = this.createSignMessage(normalizedAddress, nonce);
    const isValid = await this.verifyMessageSignature(
      normalizedAddress,
      message,
      signature,
    );

    if (!isValid) {
      throw new UnauthorizedException('Firma inválida');
    }

    // Eliminar el nonce usado (one-time use)
    await this.redis.del(nonceKey);

    // Obtener o crear usuario
    let user = await this.userRepository.findOne({
      where: { walletAddress: normalizedAddress },
    });

    if (!user) {
      user = this.userRepository.create({
        walletAddress: normalizedAddress,
        isActive: true,
        loginCount: 0,
      });
      await this.userRepository.save(user);
      this.logger.log(`New user created: ${normalizedAddress}`);
    } else if (!user.isActive) {
      throw new UnauthorizedException('Usuario desactivado');
    }

    // Actualizar estadísticas de login
    user.lastLoginAt = new Date();
    user.loginCount += 1;
    await this.userRepository.save(user);

    // Generar tokens
    const tokens = await this.generateTokens(user);

    // Guardar sesión en Redis
    await this.saveSession(user.id, tokens.refreshToken);

    this.logger.log(`User authenticated: ${normalizedAddress}`);

    return {
      ...tokens,
      user: {
        id: user.id,
        walletAddress: user.walletAddress,
        createdAt: user.createdAt,
      },
    };
  }

  /**
   * Refresca el access token usando el refresh token
   */
  async refreshToken(dto: RefreshDto): Promise<{ accessToken: string }> {
    const { refreshToken } = dto;

    try {
      // Verificar refresh token
      const payload = this.jwtService.verify(refreshToken, {
        secret: this.configService.get<string>('jwt.secret'),
      });

      // Verificar que la sesión existe en Redis
      const sessionKey = `auth:session:${payload.sub}`;
      const sessionData = await this.redis.get(sessionKey);

      if (!sessionData) {
        throw new UnauthorizedException('Sesión expirada');
      }

      const session = JSON.parse(sessionData);
      if (session.refreshToken !== refreshToken) {
        throw new UnauthorizedException('Refresh token inválido');
      }

      // Obtener usuario
      const user = await this.userRepository.findOne({
        where: { id: payload.sub },
      });

      if (!user || !user.isActive) {
        throw new UnauthorizedException('Usuario no encontrado o desactivado');
      }

      // Generar nuevo access token
      const accessToken = this.jwtService.sign(
        {
          sub: user.id,
          walletAddress: user.walletAddress,
        },
        {
          secret: this.configService.get<string>('jwt.secret'),
          expiresIn: this.configService.get<string>('jwt.expiresIn') || '24h',
        },
      );

      this.logger.debug(`Token refreshed for user: ${user.walletAddress}`);

      return { accessToken };
    } catch (error) {
      if (error instanceof UnauthorizedException) {
        throw error;
      }
      throw new UnauthorizedException('Refresh token inválido');
    }
  }

  /**
   * Cierra la sesión del usuario
   */
  async logout(userId: string): Promise<void> {
    const sessionKey = `auth:session:${userId}`;
    await this.redis.del(sessionKey);
    this.logger.debug(`Session closed for user: ${userId}`);
  }

  /**
   * Valida un access token y retorna el usuario
   */
  async validateUser(payload: any): Promise<User | null> {
    const user = await this.userRepository.findOne({
      where: { id: payload.sub },
    });

    if (!user || !user.isActive) {
      return null;
    }

    return user;
  }

  /**
   * Verifica la firma de un mensaje usando ethers
   */
  private async verifyMessageSignature(
    address: string,
    message: string,
    signature: string,
  ): Promise<boolean> {
    try {
      const recoveredAddress = ethers.verifyMessage(message, signature);
      return recoveredAddress.toLowerCase() === address.toLowerCase();
    } catch (error) {
      this.logger.warn(`Signature verification failed: ${error.message}`);
      return false;
    }
  }

  /**
   * Crea el mensaje que el usuario debe firmar
   */
  private createSignMessage(walletAddress: string, nonce: string): string {
    const domain = this.configService.get<string>('app.domain') || 'rsc.finance';
    const timestamp = Date.now();

    return `Bienvenido a ${domain}

Por favor, firma este mensaje para autenticarte.

Dirección: ${walletAddress}
Nonce: ${nonce}
Timestamp: ${timestamp}

Esta firma no te costará nada y no dará permiso para realizar ninguna transacción.`;
  }

  /**
   * Genera un nonce único
   */
  private generateNonce(): string {
    return ethers.hexlify(ethers.randomBytes(32));
  }

  /**
   * Genera access token y refresh token
   */
  private async generateTokens(user: User): Promise<{
    accessToken: string;
    refreshToken: string;
  }> {
    const payload = {
      sub: user.id,
      walletAddress: user.walletAddress,
    };

    const jwtSecret = this.configService.get<string>('jwt.secret');
    const jwtExpiresIn = this.configService.get<string>('jwt.expiresIn') || '24h';

    const accessToken = this.jwtService.sign(payload, {
      secret: jwtSecret,
      expiresIn: jwtExpiresIn,
    });

    const refreshToken = this.jwtService.sign(payload, {
      secret: jwtSecret,
      expiresIn: '7d',
    });

    return { accessToken, refreshToken };
  }

  /**
   * Guarda la sesión en Redis
   */
  private async saveSession(userId: string, refreshToken: string): Promise<void> {
    const sessionKey = `auth:session:${userId}`;
    const sessionData = {
      userId,
      refreshToken,
      createdAt: Date.now(),
    };

    await this.redis.setex(
      sessionKey,
      this.refreshTokenTtl,
      JSON.stringify(sessionData),
    );
  }

  /**
   * Verifica rate limiting para prevenir spam
   */
  private async checkRateLimit(
    walletAddress: string,
    action: 'challenge' | 'verify',
  ): Promise<void> {
    const key = `auth:ratelimit:${walletAddress}:${action}`;
    const limit = action === 'challenge' ? 10 : 5; // 10 challenges, 5 verificaciones por minuto
    const window = 60; // 1 minuto

    const current = await this.redis.incr(key);
    if (current === 1) {
      await this.redis.expire(key, window);
    }

    if (current > limit) {
      throw new BadRequestException(
        `Demasiadas solicitudes. Intenta de nuevo en ${window} segundos.`,
      );
    }
  }
}