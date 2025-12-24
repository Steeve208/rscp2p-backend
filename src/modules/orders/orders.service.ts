import {
  Injectable,
  NotFoundException,
  BadRequestException,
  ForbiddenException,
  Logger,
} from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository, FindOptionsWhere, Like } from 'typeorm';
import { Order } from '../../database/entities/order.entity';
import { OrderStatus } from '../../common/enums/order-status.enum';
import { CreateOrderDto, AcceptOrderDto, OrderResponseDto } from './dto';
import { UsersService } from '../users/users.service';
import { ReputationService } from '../reputation/reputation.service';
import { AuditService } from '../../common/audit/audit.service';
import { plainToInstance } from 'class-transformer';

@Injectable()
export class OrdersService {
  private readonly logger = new Logger(OrdersService.name);

  constructor(
    @InjectRepository(Order)
    private readonly orderRepository: Repository<Order>,
    private readonly usersService: UsersService,
    private readonly auditService: AuditService,
  ) {}

  /**
   * Crea una nueva oferta P2P
   */
  async create(sellerId: string, createOrderDto: CreateOrderDto): Promise<OrderResponseDto> {
    const {
      cryptoAmount,
      cryptoCurrency,
      fiatAmount,
      fiatCurrency,
      pricePerUnit,
      paymentMethod,
      terms,
      expiresAt,
    } = createOrderDto;

    // Calcular pricePerUnit si no se proporciona
    const calculatedPrice = pricePerUnit || fiatAmount / cryptoAmount;

    // Validar que el seller existe y está activo
    const seller = await this.usersService.getProfile(sellerId);
    if (!seller || !seller.isActive) {
      throw new BadRequestException('Vendedor no encontrado o inactivo');
    }

    // Crear la orden
    const order = this.orderRepository.create({
      sellerId,
      cryptoAmount,
      cryptoCurrency: cryptoCurrency.toUpperCase(),
      fiatAmount,
      fiatCurrency: fiatCurrency.toUpperCase(),
      pricePerUnit: calculatedPrice,
      paymentMethod,
      terms,
      expiresAt: expiresAt ? new Date(expiresAt) : null,
      status: OrderStatus.CREATED,
    });

    const savedOrder = await this.orderRepository.save(order);

    this.logger.log(
      `Order created: ${savedOrder.id} by seller ${sellerId}`,
    );

    // Auditoría
    await this.auditService.logOrderCreated(sellerId, savedOrder.id, {
      metadata: {
        cryptoAmount: savedOrder.cryptoAmount,
        cryptoCurrency: savedOrder.cryptoCurrency,
        fiatAmount: savedOrder.fiatAmount,
        fiatCurrency: savedOrder.fiatCurrency,
      },
    });

    return this.toResponseDto(savedOrder);
  }

  /**
   * Acepta una oferta (comprador acepta la oferta del vendedor)
   */
  async accept(
    orderId: string,
    buyerId: string,
    acceptOrderDto?: AcceptOrderDto,
  ): Promise<OrderResponseDto> {
    const order = await this.orderRepository.findOne({
      where: { id: orderId },
    });

    if (!order) {
      throw new NotFoundException('Orden no encontrada');
    }

    // Validar estado
    if (order.status !== OrderStatus.CREATED) {
      throw new BadRequestException(
        `No se puede aceptar una orden en estado ${order.status}`,
      );
    }

    // Validar que no sea el mismo usuario
    if (order.sellerId === buyerId) {
      throw new BadRequestException('No puedes aceptar tu propia oferta');
    }

    // Validar que el buyer existe y está activo
    const buyer = await this.usersService.getProfile(buyerId);
    if (!buyer || !buyer.isActive) {
      throw new BadRequestException('Comprador no encontrado o inactivo');
    }

    // Validar expiración
    if (order.expiresAt && new Date(order.expiresAt) < new Date()) {
      throw new BadRequestException('La orden ha expirado');
    }

    // Actualizar orden
    order.buyerId = buyerId;
    order.status = OrderStatus.AWAITING_FUNDS;
    order.acceptedAt = new Date();
    if (acceptOrderDto?.paymentMethod) {
      order.paymentMethod = acceptOrderDto.paymentMethod;
    }

    const savedOrder = await this.orderRepository.save(order);

    this.logger.log(
      `Order accepted: ${orderId} by buyer ${buyerId}`,
    );

    // Auditoría
    await this.auditService.logOrderAccepted(buyerId, orderId, {
      metadata: {
        sellerId: order.sellerId,
      },
    });

    return this.toResponseDto(savedOrder);
  }

  /**
   * Cancela una orden
   */
  async cancel(orderId: string, userId: string): Promise<OrderResponseDto> {
    const order = await this.orderRepository.findOne({
      where: { id: orderId },
    });

    if (!order) {
      throw new NotFoundException('Orden no encontrada');
    }

    // Validar que el usuario es el vendedor o comprador
    if (order.sellerId !== userId && order.buyerId !== userId) {
      throw new ForbiddenException('No tienes permiso para cancelar esta orden');
    }

    // Validar estados que permiten cancelación
    const cancellableStatuses = [
      OrderStatus.CREATED,
      OrderStatus.AWAITING_FUNDS,
    ];

    if (!cancellableStatuses.includes(order.status)) {
      throw new BadRequestException(
        `No se puede cancelar una orden en estado ${order.status}`,
      );
    }

    // Determinar quién canceló
    const cancelledBy = order.sellerId === userId ? 'SELLER' : 'BUYER';

    // Actualizar orden
    order.status = OrderStatus.REFUNDED;
    order.cancelledAt = new Date();
    order.cancelledBy = cancelledBy;

    const savedOrder = await this.orderRepository.save(order);

    // Actualizar reputation (penalización por cancelar)
    if (cancelledBy === 'SELLER') {
      await this.usersService.updateReputationScore(order.sellerId, -10);
    } else {
      await this.usersService.updateReputationScore(order.buyerId, -10);
    }

    this.logger.log(
      `Order cancelled: ${orderId} by ${cancelledBy} (${userId})`,
    );

    // Auditoría
    await this.auditService.logOrderCancelled(userId, orderId, {
      metadata: {
        cancelledBy,
        previousStatus: order.status,
      },
    });

    return this.toResponseDto(savedOrder);
  }

  /**
   * Actualiza el estado de la orden cuando se bloquean fondos on-chain
   * (puede ser automático desde blockchain o manual sin blockchain)
   */
  async markAsOnChainLocked(orderId: string, userId?: string): Promise<OrderResponseDto> {
    const order = await this.orderRepository.findOne({
      where: { id: orderId },
    });

    if (!order) {
      throw new NotFoundException('Orden no encontrada');
    }

    // Validar transición de estado usando state machine
    if (!this.isValidTransition(order.status, OrderStatus.ONCHAIN_LOCKED)) {
      throw new BadRequestException(
        `No se puede marcar como bloqueada una orden en estado ${order.status}`,
      );
    }

    // Si se proporciona userId, validar que es el comprador
    if (userId && order.buyerId !== userId) {
      throw new ForbiddenException('Solo el comprador puede marcar los fondos como bloqueados');
    }

    order.status = OrderStatus.ONCHAIN_LOCKED;
    const savedOrder = await this.orderRepository.save(order);

    this.logger.log(
      `Order marked as on-chain locked: ${orderId}${userId ? ` by user ${userId}` : ' (automatic)'}`,
    );

    return this.toResponseDto(savedOrder);
  }

  /**
   * Valida si una transición de estado es válida
   */
  private isValidTransition(from: OrderStatus, to: OrderStatus): boolean {
    const validTransitions: Record<OrderStatus, OrderStatus[]> = {
      [OrderStatus.CREATED]: [
        OrderStatus.AWAITING_FUNDS,
        OrderStatus.REFUNDED,
      ],
      [OrderStatus.AWAITING_FUNDS]: [
        OrderStatus.ONCHAIN_LOCKED,
        OrderStatus.REFUNDED,
      ],
      [OrderStatus.ONCHAIN_LOCKED]: [
        OrderStatus.COMPLETED,
        OrderStatus.REFUNDED,
        OrderStatus.DISPUTED,
      ],
      [OrderStatus.COMPLETED]: [],
      [OrderStatus.REFUNDED]: [],
      [OrderStatus.DISPUTED]: [
        OrderStatus.COMPLETED,
        OrderStatus.REFUNDED,
      ],
    };

    return validTransitions[from]?.includes(to) || false;
  }

  /**
   * Marca una orden como completada
   */
  async complete(orderId: string, completedBy: string): Promise<OrderResponseDto> {
    const order = await this.orderRepository.findOne({
      where: { id: orderId },
    });

    if (!order) {
      throw new NotFoundException('Orden no encontrada');
    }

    if (order.status !== OrderStatus.ONCHAIN_LOCKED) {
      throw new BadRequestException(
        `No se puede completar una orden en estado ${order.status}`,
      );
    }

    // Validar que quien completa es el vendedor o comprador
    if (order.sellerId !== completedBy && order.buyerId !== completedBy) {
      throw new ForbiddenException('No tienes permiso para completar esta orden');
    }

    order.status = OrderStatus.COMPLETED;
    order.completedAt = new Date();
    const savedOrder = await this.orderRepository.save(order);

    // Actualizar reputation (bonificación por completar)
    await this.usersService.updateReputationScore(order.sellerId, 5);
    if (order.buyerId) {
      await this.usersService.updateReputationScore(order.buyerId, 5);
    }

    this.logger.log(`Order completed: ${orderId} by ${completedBy}`);

    return this.toResponseDto(savedOrder);
  }

  /**
   * Marca una orden como disputada
   */
  async markAsDisputed(orderId: string): Promise<OrderResponseDto> {
    const order = await this.orderRepository.findOne({
      where: { id: orderId },
    });

    if (!order) {
      throw new NotFoundException('Orden no encontrada');
    }

    const disputableStatuses = [
      OrderStatus.AWAITING_FUNDS,
      OrderStatus.ONCHAIN_LOCKED,
    ];

    if (!disputableStatuses.includes(order.status)) {
      throw new BadRequestException(
        `No se puede disputar una orden en estado ${order.status}`,
      );
    }

    order.status = OrderStatus.DISPUTED;
    const savedOrder = await this.orderRepository.save(order);

    this.logger.log(`Order marked as disputed: ${orderId}`);

    return this.toResponseDto(savedOrder);
  }

  /**
   * Lista todas las órdenes con filtros
   */
  async findAll(
    page: number = 1,
    limit: number = 20,
    status?: OrderStatus,
    sellerId?: string,
    buyerId?: string,
    cryptoCurrency?: string,
    fiatCurrency?: string,
  ): Promise<{
    data: OrderResponseDto[];
    total: number;
    page: number;
    limit: number;
    totalPages: number;
  }> {
    const skip = (page - 1) * limit;
    const where: FindOptionsWhere<Order> = {};

    if (status) {
      where.status = status;
    }
    if (sellerId) {
      where.sellerId = sellerId;
    }
    if (buyerId) {
      where.buyerId = buyerId;
    }
    if (cryptoCurrency) {
      where.cryptoCurrency = cryptoCurrency.toUpperCase();
    }
    if (fiatCurrency) {
      where.fiatCurrency = fiatCurrency.toUpperCase();
    }

    const [orders, total] = await this.orderRepository.findAndCount({
      where,
      skip,
      take: limit,
      order: {
        createdAt: 'DESC',
      },
    });

    return {
      data: orders.map((order) => this.toResponseDto(order)),
      total,
      page,
      limit,
      totalPages: Math.ceil(total / limit),
    };
  }

  /**
   * Obtiene una orden por ID
   */
  async findOne(id: string): Promise<OrderResponseDto> {
    const order = await this.orderRepository.findOne({
      where: { id },
    });

    if (!order) {
      throw new NotFoundException('Orden no encontrada');
    }

    return this.toResponseDto(order);
  }

  /**
   * Obtiene las órdenes de un usuario (como vendedor o comprador)
   */
  async findByUser(
    userId: string,
    role: 'seller' | 'buyer' | 'both' = 'both',
    status?: OrderStatus,
  ): Promise<OrderResponseDto[]> {
    const where: FindOptionsWhere<Order>[] = [];

    if (role === 'seller' || role === 'both') {
      where.push({ sellerId: userId, ...(status && { status }) });
    }
    if (role === 'buyer' || role === 'both') {
      where.push({ buyerId: userId, ...(status && { status }) });
    }

    const orders = await this.orderRepository.find({
      where: where.length > 1 ? (where as any) : where[0],
      order: {
        createdAt: 'DESC',
      },
    });

    return orders.map((order) => this.toResponseDto(order));
  }

  /**
   * Convierte Order a DTO de respuesta
   */
  private toResponseDto(order: Order): OrderResponseDto {
    return plainToInstance(OrderResponseDto, {
      id: order.id,
      seller_id: order.sellerId,
      buyer_id: order.buyerId,
      crypto_amount: Number(order.cryptoAmount),
      crypto_currency: order.cryptoCurrency,
      fiat_amount: Number(order.fiatAmount),
      fiat_currency: order.fiatCurrency,
      price_per_unit: order.pricePerUnit ? Number(order.pricePerUnit) : null,
      status: order.status,
      escrow_id: order.escrowId,
      payment_method: order.paymentMethod,
      terms: order.terms,
      expires_at: order.expiresAt,
      accepted_at: order.acceptedAt,
      completed_at: order.completedAt,
      cancelled_at: order.cancelledAt,
      cancelled_by: order.cancelledBy,
      created_at: order.createdAt,
      updated_at: order.updatedAt,
    });
  }
}