import {
  Controller,
  Get,
  Post,
  Put,
  Param,
  Body,
  Query,
  UseGuards,
  HttpCode,
  HttpStatus,
  ParseEnumPipe,
} from '@nestjs/common';
import { DisputesService } from './disputes.service';
import { JwtAuthGuard } from '../../common/guards/jwt-auth.guard';
import { CurrentUser } from '../../common/decorators/current-user.decorator';
import { User } from '../../database/entities/user.entity';
import { CreateDisputeDto, AddEvidenceDto, ResolveDisputeDto } from './dto';
import { DisputeStatus } from '../../common/enums/dispute-status.enum';

@Controller('disputes')
export class DisputesController {
  constructor(private readonly disputesService: DisputesService) {}

  /**
   * Abre una nueva disputa
   * POST /api/disputes
   */
  @Post()
  @UseGuards(JwtAuthGuard)
  @HttpCode(HttpStatus.CREATED)
  async create(
    @CurrentUser() user: User,
    @Body() createDisputeDto: CreateDisputeDto,
  ) {
    return this.disputesService.create(user.id, createDisputeDto);
  }

  /**
   * Lista todas las disputas con filtros
   * GET /api/disputes?status=OPEN&orderId=xxx&userId=xxx
   */
  @Get()
  @HttpCode(HttpStatus.OK)
  async findAll(
    @Query('status', new ParseEnumPipe(DisputeStatus, { optional: true }))
    status?: DisputeStatus,
    @Query('orderId') orderId?: string,
    @Query('userId') userId?: string,
  ) {
    return this.disputesService.findAll(status, orderId, userId);
  }

  /**
   * Obtiene una disputa por ID
   * GET /api/disputes/:id
   */
  @Get(':id')
  @HttpCode(HttpStatus.OK)
  async findOne(@Param('id') id: string) {
    return this.disputesService.findOne(id);
  }

  /**
   * Agrega evidencia off-chain a una disputa
   * POST /api/disputes/:id/evidence
   */
  @Post(':id/evidence')
  @UseGuards(JwtAuthGuard)
  @HttpCode(HttpStatus.CREATED)
  async addEvidence(
    @Param('id') id: string,
    @CurrentUser() user: User,
    @Body() addEvidenceDto: AddEvidenceDto,
  ) {
    return this.disputesService.addEvidence(id, user.id, addEvidenceDto);
  }

  /**
   * Resuelve una disputa
   * PUT /api/disputes/:id/resolve
   * NOTA: La resolución final siempre depende del escrow
   */
  @Put(':id/resolve')
  @UseGuards(JwtAuthGuard)
  @HttpCode(HttpStatus.OK)
  async resolve(
    @Param('id') id: string,
    @Body() resolveDto: ResolveDisputeDto,
  ) {
    return this.disputesService.resolve(id, resolveDto);
  }

  /**
   * Cierra una disputa (después de resolución del escrow)
   * PUT /api/disputes/:id/close
   */
  @Put(':id/close')
  @UseGuards(JwtAuthGuard)
  @HttpCode(HttpStatus.OK)
  async close(
    @Param('id') id: string,
    @Body('escrowResolution') escrowResolution: string,
  ) {
    return this.disputesService.close(id, escrowResolution);
  }

  /**
   * Escala una disputa
   * PUT /api/disputes/:id/escalate
   */
  @Put(':id/escalate')
  @UseGuards(JwtAuthGuard)
  @HttpCode(HttpStatus.OK)
  async escalate(@Param('id') id: string) {
    return this.disputesService.escalate(id);
  }

  /**
   * Obtiene disputas próximas a expirar
   * GET /api/disputes/expiring?hours=24
   */
  @Get('expiring')
  @HttpCode(HttpStatus.OK)
  async getExpiringDisputes(@Query('hours') hours?: string) {
    const hoursNum = hours ? parseInt(hours, 10) : 24;
    return this.disputesService.getExpiringDisputes(hoursNum);
  }
}