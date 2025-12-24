import {
  Controller,
  Get,
  Post,
  Body,
  Param,
  Query,
  HttpCode,
  HttpStatus,
  ParseIntPipe,
  DefaultValuePipe,
} from '@nestjs/common';
import { ReputationService } from './reputation.service';
import { ApplyPenaltyDto, ApplyBonusDto } from './dto';

@Controller('reputation')
export class ReputationController {
  constructor(private readonly reputationService: ReputationService) {}

  /**
   * Obtiene la reputation de un usuario
   * GET /api/reputation/:userId
   */
  @Get(':userId')
  @HttpCode(HttpStatus.OK)
  async getUserReputation(@Param('userId') userId: string) {
    return this.reputationService.getUserReputation(userId);
  }

  /**
   * Obtiene el historial de eventos de reputation
   * GET /api/reputation/:userId/history?limit=50
   */
  @Get(':userId/history')
  @HttpCode(HttpStatus.OK)
  async getReputationHistory(
    @Param('userId') userId: string,
    @Query('limit', new DefaultValuePipe(50), ParseIntPipe) limit: number,
  ) {
    return this.reputationService.getReputationHistory(userId, limit);
  }

  /**
   * Obtiene estadísticas de reputation
   * GET /api/reputation/:userId/stats
   */
  @Get(':userId/stats')
  @HttpCode(HttpStatus.OK)
  async getReputationStats(@Param('userId') userId: string) {
    return this.reputationService.getReputationStats(userId);
  }

  /**
   * Re-calcula la reputation basado en eventos históricos
   * POST /api/reputation/:userId/recalculate
   */
  @Post(':userId/recalculate')
  @HttpCode(HttpStatus.OK)
  async recalculateReputation(@Param('userId') userId: string) {
    return this.reputationService.recalculateReputation(userId);
  }

  /**
   * Obtiene el ranking de usuarios por reputation
   * GET /api/reputation/ranking?limit=100
   */
  @Get('ranking')
  @HttpCode(HttpStatus.OK)
  async getRanking(
    @Query('limit', new DefaultValuePipe(100), ParseIntPipe) limit: number,
  ) {
    return this.reputationService.getRanking(limit);
  }

  /**
   * Aplica una penalización manual
   * POST /api/reputation/penalty
   */
  @Post('penalty')
  @HttpCode(HttpStatus.OK)
  async applyPenalty(@Body() dto: ApplyPenaltyDto) {
    return this.reputationService.applyPenalty(
      dto.userId,
      dto.reason,
      dto.orderId,
      dto.disputeId,
    );
  }

  /**
   * Aplica un bonus manual
   * POST /api/reputation/bonus
   */
  @Post('bonus')
  @HttpCode(HttpStatus.OK)
  async applyBonus(@Body() dto: ApplyBonusDto) {
    return this.reputationService.applyBonus(
      dto.userId,
      dto.reason,
      dto.orderId,
      dto.disputeId,
    );
  }
}