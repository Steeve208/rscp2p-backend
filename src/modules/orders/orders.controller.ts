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
  ParseIntPipe,
  DefaultValuePipe,
  ParseEnumPipe,
} from '@nestjs/common';
import { OrdersService } from './orders.service';
import { JwtAuthGuard } from '../../common/guards/jwt-auth.guard';
import { RateLimitGuard, RateLimit } from '../../common/guards/rate-limit.guard';
import { CurrentUser } from '../../common/decorators/current-user.decorator';
import { User } from '../../database/entities/user.entity';
import { CreateOrderDto, AcceptOrderDto } from './dto';
import { OrderStatus } from '../../common/enums/order-status.enum';
import { PaginationDto, ApiResponseDto } from '../../common/dto';

/**
 * API pública de órdenes
 * 
 * Rol: API pública de órdenes
 * 
 * Contiene:
 * - Endpoints REST (create, accept, cancel, status)
 * - Gestión de ofertas P2P
 * - Búsqueda y filtrado de órdenes
 * 
 * Se conecta con:
 * - orders.service.ts (única conexión permitida)
 * 
 * NUNCA debe:
 * - Hablar con la blockchain directamente
 * - Ejecutar transacciones blockchain
 * - Usar providers de blockchain
 * - Importar módulos de blockchain
 * 
 * La lógica de blockchain se maneja en:
 * - blockchain/ module (escucha eventos)
 * - escrow/ module (mapea order_id ↔ escrow_id)
 */
@Controller('orders')
export class OrdersController {
  constructor(private readonly ordersService: OrdersService) {}

  /**
   * Crea una nueva oferta P2P
   * POST /api/orders
   * 
   * Requiere autenticación
   * Rate limit: 10 requests/minuto
   */
  @Post()
  @UseGuards(JwtAuthGuard, RateLimitGuard)
  @RateLimit(10, 60) // 10 requests por minuto
  @HttpCode(HttpStatus.CREATED)
  async create(
    @CurrentUser() user: User,
    @Body() createOrderDto: CreateOrderDto,
  ) {
    const order = await this.ordersService.create(user.id, createOrderDto);
    return ApiResponseDto.success(order, 'Orden creada exitosamente');
  }

  /**
   * Lista todas las órdenes con filtros (público)
   * GET /api/orders?page=1&limit=20&status=CREATED&cryptoCurrency=BTC&fiatCurrency=USD
   * 
   * Público (no requiere autenticación)
   * Rate limit: 30 requests/minuto
   */
  @Get()
  @UseGuards(RateLimitGuard)
  @RateLimit(30, 60) // 30 requests por minuto
  @HttpCode(HttpStatus.OK)
  async findAll(
    @Query('page', new DefaultValuePipe(1), ParseIntPipe) page: number,
    @Query('limit', new DefaultValuePipe(20), ParseIntPipe) limit: number,
    @Query('status', new ParseEnumPipe(OrderStatus, { optional: true }))
    status?: OrderStatus,
    @Query('sellerId') sellerId?: string,
    @Query('buyerId') buyerId?: string,
    @Query('cryptoCurrency') cryptoCurrency?: string,
    @Query('fiatCurrency') fiatCurrency?: string,
  ) {
    const result = await this.ordersService.findAll(
      page,
      limit,
      status,
      sellerId,
      buyerId,
      cryptoCurrency,
      fiatCurrency,
    );
    return ApiResponseDto.success(result, 'Órdenes obtenidas exitosamente');
  }

  /**
   * Obtiene una orden por ID (público)
   * GET /api/orders/:id
   * 
   * Público (no requiere autenticación)
   * Rate limit: 30 requests/minuto
   */
  @Get(':id')
  @UseGuards(RateLimitGuard)
  @RateLimit(30, 60) // 30 requests por minuto
  @HttpCode(HttpStatus.OK)
  async findOne(@Param('id') id: string) {
    const order = await this.ordersService.findOne(id);
    return ApiResponseDto.success(order, 'Orden obtenida exitosamente');
  }

  /**
   * Obtiene el estado de una orden
   * GET /api/orders/:id/status
   * 
   * Público (no requiere autenticación)
   * Rate limit: 30 requests/minuto
   */
  @Get(':id/status')
  @UseGuards(RateLimitGuard)
  @RateLimit(30, 60) // 30 requests por minuto
  @HttpCode(HttpStatus.OK)
  async getStatus(@Param('id') id: string) {
    const order = await this.ordersService.findOne(id);
    return ApiResponseDto.success(
      {
        id: order.id,
        status: order.status,
        escrowId: order.escrowId,
        updatedAt: order.updatedAt,
      },
      'Estado de orden obtenido exitosamente',
    );
  }

  /**
   * Acepta una oferta (comprador acepta)
   * PUT /api/orders/:id/accept
   * 
   * Requiere autenticación
   * Rate limit: 5 requests/minuto
   */
  @Put(':id/accept')
  @UseGuards(JwtAuthGuard, RateLimitGuard)
  @RateLimit(5, 60) // 5 requests por minuto
  @HttpCode(HttpStatus.OK)
  async accept(
    @Param('id') id: string,
    @CurrentUser() user: User,
    @Body() acceptOrderDto?: AcceptOrderDto,
  ) {
    const order = await this.ordersService.accept(id, user.id, acceptOrderDto);
    return ApiResponseDto.success(order, 'Orden aceptada exitosamente');
  }

  /**
   * Cancela una orden
   * PUT /api/orders/:id/cancel
   * 
   * Requiere autenticación
   * Rate limit: 10 requests/minuto
   */
  @Put(':id/cancel')
  @UseGuards(JwtAuthGuard, RateLimitGuard)
  @RateLimit(10, 60) // 10 requests por minuto
  @HttpCode(HttpStatus.OK)
  async cancel(@Param('id') id: string, @CurrentUser() user: User) {
    const order = await this.ordersService.cancel(id, user.id);
    return ApiResponseDto.success(order, 'Orden cancelada exitosamente');
  }

  /**
   * Obtiene las órdenes del usuario autenticado
   * GET /api/orders/me?role=seller&status=CREATED
   * 
   * Requiere autenticación
   * Rate limit: 20 requests/minuto
   */
  @Get('me')
  @UseGuards(JwtAuthGuard, RateLimitGuard)
  @RateLimit(20, 60) // 20 requests por minuto
  @HttpCode(HttpStatus.OK)
  async findMyOrders(
    @CurrentUser() user: User,
    @Query('role') role?: 'seller' | 'buyer' | 'both',
    @Query('status', new ParseEnumPipe(OrderStatus, { optional: true }))
    status?: OrderStatus,
    @Query('page', new DefaultValuePipe(1), ParseIntPipe) page?: number,
    @Query('limit', new DefaultValuePipe(20), ParseIntPipe) limit?: number,
  ) {
    const result = await this.ordersService.findByUser(
      user.id,
      role || 'both',
      status,
    );
    return ApiResponseDto.success(result, 'Tus órdenes obtenidas exitosamente');
  }

  /**
   * Marca una orden como completada
   * PUT /api/orders/:id/complete
   * 
   * Requiere autenticación
   * Rate limit: 5 requests/minuto
   */
  @Put(':id/complete')
  @UseGuards(JwtAuthGuard, RateLimitGuard)
  @RateLimit(5, 60) // 5 requests por minuto
  @HttpCode(HttpStatus.OK)
  async complete(@Param('id') id: string, @CurrentUser() user: User) {
    const order = await this.ordersService.complete(id, user.id);
    return ApiResponseDto.success(order, 'Orden completada exitosamente');
  }

  /**
   * Marca una orden como disputada
   * PUT /api/orders/:id/dispute
   * 
   * Requiere autenticación
   * Rate limit: 3 requests/minuto
   * 
   * NOTA: La apertura real de disputa se hace en /api/disputes
   * Este endpoint solo marca la orden como disputada
   */
  @Put(':id/dispute')
  @UseGuards(JwtAuthGuard, RateLimitGuard)
  @RateLimit(3, 60) // 3 requests por minuto
  @HttpCode(HttpStatus.OK)
  async dispute(@Param('id') id: string, @CurrentUser() user: User) {
    // Validar que el usuario es parte de la orden (se hace en el servicio)
    const order = await this.ordersService.markAsDisputed(id);
    return ApiResponseDto.success(order, 'Orden marcada como disputada');
  }

  /**
   * Marca una orden como bloqueada (fondos bloqueados)
   * PUT /api/orders/:id/mark-locked
   * 
   * Requiere autenticación
   * Rate limit: 5 requests/minuto
   * 
   * Permite marcar manualmente que los fondos están bloqueados
   * Útil cuando no hay blockchain disponible
   */
  @Put(':id/mark-locked')
  @UseGuards(JwtAuthGuard, RateLimitGuard)
  @RateLimit(5, 60) // 5 requests por minuto
  @HttpCode(HttpStatus.OK)
  async markLocked(@Param('id') id: string, @CurrentUser() user: User) {
    const order = await this.ordersService.markAsOnChainLocked(id, user.id);
    return ApiResponseDto.success(order, 'Orden marcada como bloqueada');
  }
}
