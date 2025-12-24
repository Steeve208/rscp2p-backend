import {
  Injectable,
  NotFoundException,
  BadRequestException,
  Logger,
} from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository, FindOptionsWhere, Like } from 'typeorm';
import { User } from '../../database/entities/user.entity';
import { UserResponseDto } from './dto';
import { plainToInstance } from 'class-transformer';

@Injectable()
export class UsersService {
  private readonly logger = new Logger(UsersService.name);

  constructor(
    @InjectRepository(User)
    private readonly userRepository: Repository<User>,
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
    const normalizedAddress = walletAddress.toLowerCase();
    const user = await this.userRepository.findOne({
      where: { walletAddress: normalizedAddress },
    });

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
    reputationScore: number;
    createdAt: Date;
    rank?: number;
  }> {
    const user = await this.userRepository.findOne({
      where: { walletAddress: walletAddress.toLowerCase() },
    });

    if (!user) {
      throw new NotFoundException('Usuario no encontrado');
    }

    // Calcular ranking (usuarios con mayor reputation score)
    const rank = await this.userRepository
      .createQueryBuilder('user')
      .where('user.isActive = :isActive', { isActive: true })
      .andWhere('user.reputationScore > :score', {
        score: user.reputationScore,
      })
      .getCount() + 1;

    return {
      walletAddress: user.walletAddress,
      reputationScore: Number(user.reputationScore),
      createdAt: user.createdAt,
      rank,
    };
  }

  /**
   * Convierte un User a DTO público (sin información personal)
   */
  private toPublicDto(user: User): UserResponseDto {
    return plainToInstance(UserResponseDto, {
      id: user.id,
      wallet_address: user.walletAddress,
      reputation_score: Number(user.reputationScore),
      created_at: user.createdAt,
    });
  }
}