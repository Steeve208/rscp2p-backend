import {
  Controller,
  Post,
  Body,
  HttpCode,
  HttpStatus,
  UseGuards,
  Get,
} from '@nestjs/common';
import { AuthService } from './auth.service';
import { ChallengeDto, VerifyDto, RefreshDto } from './dto';
import { JwtAuthGuard } from '../../common/guards/jwt-auth.guard';
import { CurrentUser } from '../../common/decorators/current-user.decorator';
import { User } from '../../database/entities/user.entity';

@Controller('auth')
export class AuthController {
  constructor(private readonly authService: AuthService) {}

  /**
   * Genera un challenge (nonce) para que el usuario lo firme
   * POST /api/auth/challenge
   */
  @Post('challenge')
  @HttpCode(HttpStatus.OK)
  async generateChallenge(@Body() dto: ChallengeDto) {
    return this.authService.generateChallenge(dto);
  }

  /**
   * Verifica la firma y autentica al usuario
   * POST /api/auth/verify
   */
  @Post('verify')
  @HttpCode(HttpStatus.OK)
  async verifySignature(@Body() dto: VerifyDto) {
    return this.authService.verifySignature(dto);
  }

  /**
   * Refresca el access token
   * POST /api/auth/refresh
   */
  @Post('refresh')
  @HttpCode(HttpStatus.OK)
  async refreshToken(@Body() dto: RefreshDto) {
    return this.authService.refreshToken(dto);
  }

  /**
   * Obtiene el perfil del usuario autenticado
   * GET /api/auth/me
   */
  @Get('me')
  @UseGuards(JwtAuthGuard)
  @HttpCode(HttpStatus.OK)
  async getProfile(@CurrentUser() user: User) {
    return {
      id: user.id,
      walletAddress: user.walletAddress,
      isActive: user.isActive,
      loginCount: user.loginCount,
      lastLoginAt: user.lastLoginAt,
      createdAt: user.createdAt,
    };
  }

  /**
   * Cierra la sesión del usuario
   * POST /api/auth/logout
   */
  @Post('logout')
  @UseGuards(JwtAuthGuard)
  @HttpCode(HttpStatus.OK)
  async logout(@CurrentUser() user: User) {
    await this.authService.logout(user.id);
    return { message: 'Sesión cerrada exitosamente' };
  }
}