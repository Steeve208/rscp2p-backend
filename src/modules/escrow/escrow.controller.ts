import {
  Controller,
  Get,
  Post,
  Put,
  Param,
  Body,
  Query,
  HttpCode,
  HttpStatus,
  ParseEnumPipe,
} from '@nestjs/common';
import { EscrowService } from './escrow.service';
import { CreateEscrowDto, UpdateEscrowDto } from './dto';
import { EscrowStatus } from '../../common/enums/escrow-status.enum';

@Controller('escrow')
export class EscrowController {
  constructor(private readonly escrowService: EscrowService) {}

  /**
   * Crea el mapeo order_id ↔ escrow_id
   * POST /api/escrow
   */
  @Post()
  @HttpCode(HttpStatus.CREATED)
  async create(@Body() createEscrowDto: CreateEscrowDto) {
    return this.escrowService.create(createEscrowDto);
  }

  /**
   * Obtiene un escrow por ID
   * GET /api/escrow/:id
   */
  @Get(':id')
  @HttpCode(HttpStatus.OK)
  async findOne(@Param('id') id: string) {
    return this.escrowService.findOne(id);
  }

  /**
   * Obtiene el escrow por order_id
   * GET /api/escrow/order/:orderId
   */
  @Get('order/:orderId')
  @HttpCode(HttpStatus.OK)
  async findByOrderId(@Param('orderId') orderId: string) {
    return this.escrowService.findByOrderId(orderId);
  }

  /**
   * Obtiene el escrow por escrow_id (blockchain)
   * GET /api/escrow/blockchain/:escrowId
   */
  @Get('blockchain/:escrowId')
  @HttpCode(HttpStatus.OK)
  async findByEscrowId(@Param('escrowId') escrowId: string) {
    return this.escrowService.findByEscrowId(escrowId);
  }

  /**
   * Obtiene el mapeo order_id ↔ escrow_id
   * GET /api/escrow/mapping?orderId=xxx o ?escrowId=xxx
   */
  @Get('mapping')
  @HttpCode(HttpStatus.OK)
  async getMapping(
    @Query('orderId') orderId?: string,
    @Query('escrowId') escrowId?: string,
  ) {
    return this.escrowService.getMapping(orderId, escrowId);
  }

  /**
   * Valida la consistencia entre orden y escrow
   * GET /api/escrow/validate/:orderId
   */
  @Get('validate/:orderId')
  @HttpCode(HttpStatus.OK)
  async validateConsistency(@Param('orderId') orderId: string) {
    return this.escrowService.validateConsistency(orderId);
  }

  /**
   * Lista todos los escrows con filtros
   * GET /api/escrow?orderId=xxx&escrowId=xxx&status=LOCKED
   */
  @Get()
  @HttpCode(HttpStatus.OK)
  async findAll(
    @Query('orderId') orderId?: string,
    @Query('escrowId') escrowId?: string,
    @Query('status', new ParseEnumPipe(EscrowStatus, { optional: true }))
    status?: EscrowStatus,
  ) {
    return this.escrowService.findAll(orderId, escrowId, status);
  }

  /**
   * Actualiza el estado del escrow (cuando se ejecutan transacciones en blockchain)
   * PUT /api/escrow/:escrowId
   */
  @Put(':escrowId')
  @HttpCode(HttpStatus.OK)
  async update(
    @Param('escrowId') escrowId: string,
    @Body() updateEscrowDto: UpdateEscrowDto,
  ) {
    return this.escrowService.update(escrowId, updateEscrowDto);
  }
}