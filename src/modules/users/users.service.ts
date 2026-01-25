import {
  Injectable,
  NotFoundException,
  BadRequestException,
  Logger,
} from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository, FindOptionsWhere, Like } from 'typeorm';
import { User } from '../../database/entities/user.entity';
import { Order } from '../../database/entities/order.entity';
import { OrderStatus } from '../../common/enums/order-status.enum';
import { UserResponseDto } from './dto';
import { plainToInstance } from 'class-transformer';

@Injectable()
export class UsersService {
  private readonly logger = new Logger(UsersService.name);

  constructor(
    @InjectRepository(User)
    private readonly userRepository: Repository<User>,
    @InjectRepository(Order)
    private readonly orderRepository: Repository<Order>,
  ) {}

  /**
   * Encuentra un usuario por ID (solo datos públicos)
   */
  async findOneById(id: string): Promise<UserResponseDto> {
    const user = await this.userRepository.findOne({
      where: { id },
    });

    if (!user) {
      throw new NotFoundException('Usuario no encontrado');
    }

    return this.toPublicDto(user);
  }

  /**
   * Encuentra un usuario por wallet address (solo datos públicos)
   */
  async findOneByWallet(walletAddress: string): Promise<UserResponseDto> {
    const user = await this.userRepository
      .createQueryBuilder('user')
      .where('LOWER(user.walletAddress) = LOWER(:address)', {
        address: walletAddress,
      })
      .getOne();

    if (!user) {
      throw new NotFoundException('Usuario no encontrado');
    }

    return this.toPublicDto(user);
  }

  /**
   * Lista usuarios con paginación (solo datos públicos)
   */
  async findAll(
    page: number = 1,
    limit: number = 20,
    search?: string,
  ): Promise<{
    data: UserResponseDto[];
    total: number;
    page: number;
    limit: number;
    totalPages: number;
  }> {
    const skip = (page - 1) * limit;
    const where: FindOptionsWhere<User> = { isActive: true };

    if (search) {
      where.walletAddress = Like(`%${search.toLowerCase()}%`) as any;
    }

    const [users, total] = await this.userRepository.findAndCount({
      where,
      skip,
      take: limit,
      order: {
        reputationScore: 'DESC',
        createdAt: 'DESC',
      },
    });

    return {
      data: users.map((user) => this.toPublicDto(user)),
      total,
      page,
      limit,
      totalPages: Math.ceil(total / limit),
    };
  }

  /**
   * Obtiene el perfil completo del usuario (solo para el propio usuario)
   */
  async getProfile(userId: string): Promise<User> {
    const user = await this.userRepository.findOne({
      where: { id: userId },
    });

    if (!user) {
      throw new NotFoundException('Usuario no encontrado');
    }

    return user;
  }

  /**
   * Obtiene el perfil público del usuario autenticado con stats
   */
  async getProfileWithStats(userId: string): Promise<{
    id: string;
    walletAddress: string;
    reputationScore: number;
    isActive: boolean;
    loginCount: number;
    lastLoginAt: Date;
    createdAt: Date;
    totalOrders: number;
    completedOrders: number;
    cancelledOrders: number;
    disputedOrders: number;
    averageRating: number;
  }> {
    const user = await this.getProfile(userId);
    const stats = await this.computeStats(user.id);

    return {
      ...this.toPublicDto(user),
      ...stats,
      walletAddress: user.walletAddress,
    };
  }

  /**
   * Actualiza el reputation score de un usuario
   * Usado internamente por otros módulos (orders, disputes, etc.)
   */
  async updateReputationScore(
    userId: string,
    scoreDelta: number,
  ): Promise<User> {
    const user = await this.userRepository.findOne({
      where: { id: userId },
    });

    if (!user) {
      throw new NotFoundException('Usuario no encontrado');
    }

    // Calcular nuevo score con límites
    const newScore = Math.max(
      -100,
      Math.min(100, Number(user.reputationScore) + scoreDelta),
    );

    user.reputationScore = newScore;
    await this.userRepository.save(user);

    this.logger.debug(
      `Reputation updated for user ${userId}: ${user.reputationScore} -> ${newScore}`,
    );

    return user;
  }

  /**
   * Establece el reputation score directamente
   */
  async setReputationScore(userId: string, score: number): Promise<User> {
    if (score < -100 || score > 100) {
      throw new BadRequestException(
        'El reputation score debe estar entre -100 y 100',
      );
    }

    const user = await this.userRepository.findOne({
      where: { id: userId },
    });

    if (!user) {
      throw new NotFoundException('Usuario no encontrado');
    }

    user.reputationScore = score;
    await this.userRepository.save(user);

    this.logger.debug(
      `Reputation set for user ${userId}: ${score}`,
    );

    return user;
  }

  /**
   * Obtiene el ranking de usuarios por reputation
   */
  async getRanking(limit: number = 100): Promise<UserResponseDto[]> {
    const users = await this.userRepository.find({
      where: { isActive: true },
      order: {
        reputationScore: 'DESC',
        createdAt: 'ASC',
      },
      take: limit,
    });

    return users.map((user) => this.toPublicDto(user));
  }

  /**
   * Obtiene estadísticas de un usuario (solo datos públicos)
   */
  async getStats(walletAddress: string): Promise<{
    walletAddress: string;
    totalOrders: number;
    completedOrders: number;
    cancelledOrders: number;
    disputedOrders: number;
    averageRating: number;
  }> {
    const user = await this.userRepository
      .createQueryBuilder('user')
      .where('LOWER(user.walletAddress) = LOWER(:address)', {
        address: walletAddress,
      })
      .getOne();

    if (!user) {
      throw new NotFoundException('Usuario no encontrado');
    }

    const stats = await this.computeStats(user.id);

    return {
      walletAddress: user.walletAddress,
      ...stats,
    };
  }

  /**
   * Convierte un User a DTO público (sin información personal)
   */
  private toPublicDto(user: User): UserResponseDto {
    return plainToInstance(UserResponseDto, {
      id: user.id,
      walletAddress: user.walletAddress,
      reputationScore: Number(user.reputationScore),
      isActive: user.isActive,
      loginCount: user.loginCount,
      lastLoginAt: user.lastLoginAt,
      createdAt: user.createdAt,
    });
  }

  /**
   * Calcula estadísticas agregadas del usuario sobre órdenes
   */
  private async computeStats(userId: string): Promise<{
    totalOrders: number;
    completedOrders: number;
    cancelledOrders: number;
    disputedOrders: number;
    averageRating: number;
  }> {
    const totalOrders = await this.orderRepository.count({
      where: [{ sellerId: userId }, { buyerId: userId }],
    });

    const completedOrders = await this.orderRepository.count({
      where: [
        { sellerId: userId, status: OrderStatus.COMPLETED },
        { buyerId: userId, status: OrderStatus.COMPLETED },
      ],
    });

    const cancelledOrders = await this.orderRepository.count({
      where: [
        { sellerId: userId, status: OrderStatus.REFUNDED },
        { buyerId: userId, status: OrderStatus.REFUNDED },
      ],
    });

    const disputedOrders = await this.orderRepository.count({
      where: [
        { sellerId: userId, status: OrderStatus.DISPUTED },
        { buyerId: userId, status: OrderStatus.DISPUTED },
      ],
    });

    return {
      totalOrders,
      completedOrders,
      cancelledOrders,
      disputedOrders,
      averageRating: 0,
    };
  }
}