import {
  Controller,
  Get,
  Query,
  Param,
  UseGuards,
  HttpCode,
  HttpStatus,
  ParseIntPipe,
  DefaultValuePipe,
} from '@nestjs/common';
import { UsersService } from './users.service';
import { JwtAuthGuard } from '../../common/guards/jwt-auth.guard';
import { CurrentUser } from '../../common/decorators/current-user.decorator';
import { User } from '../../database/entities/user.entity';
import { UserResponseDto } from './dto';

@Controller('users')
export class UsersController {
  constructor(private readonly usersService: UsersService) {}

  /**
   * Lista usuarios con paginación (público)
   * GET /api/users?page=1&limit=20&search=0x...
   */
  @Get()
  @HttpCode(HttpStatus.OK)
  async findAll(
    @Query('page', new DefaultValuePipe(1), ParseIntPipe) page: number,
    @Query('limit', new DefaultValuePipe(20), ParseIntPipe) limit: number,
    @Query('search') search?: string,
  ) {
    return this.usersService.findAll(page, limit, search);
  }

  /**
   * Obtiene un usuario por ID (público)
   * GET /api/users/:id
   */
  @Get(':id')
  @HttpCode(HttpStatus.OK)
  async findOneById(@Param('id') id: string): Promise<UserResponseDto> {
    return this.usersService.findOneById(id);
  }

  /**
   * Obtiene un usuario por wallet address (público)
   * GET /api/users/wallet/:address
   */
  @Get('wallet/:address')
  @HttpCode(HttpStatus.OK)
  async findOneByWallet(
    @Param('address') address: string,
  ): Promise<UserResponseDto> {
    return this.usersService.findOneByWallet(address);
  }

  /**
   * Obtiene el ranking de usuarios por reputation (público)
   * GET /api/users/ranking?limit=100
   */
  @Get('ranking')
  @HttpCode(HttpStatus.OK)
  async getRanking(
    @Query('limit', new DefaultValuePipe(100), ParseIntPipe) limit: number,
  ): Promise<UserResponseDto[]> {
    return this.usersService.getRanking(limit);
  }

  /**
   * Obtiene estadísticas de un usuario (público)
   * GET /api/users/stats/:address
   */
  @Get('stats/:address')
  @HttpCode(HttpStatus.OK)
  async getStats(@Param('address') address: string) {
    return this.usersService.getStats(address);
  }

  /**
   * Obtiene el perfil completo del usuario autenticado (protegido)
   * GET /api/users/me/profile
   */
  @Get('me/profile')
  @UseGuards(JwtAuthGuard)
  @HttpCode(HttpStatus.OK)
  async getProfile(@CurrentUser() user: User) {
    const profile = await this.usersService.getProfile(user.id);
    
    // Retornar solo datos públicos incluso en el perfil propio
    // Nunca exponer información personal
    return {
      id: profile.id,
      walletAddress: profile.walletAddress,
      reputationScore: Number(profile.reputationScore),
      createdAt: profile.createdAt,
      isActive: profile.isActive,
      lastLoginAt: profile.lastLoginAt,
      loginCount: profile.loginCount,
    };
  }
}