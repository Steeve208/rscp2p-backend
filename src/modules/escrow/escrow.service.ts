import {
  Injectable,
  NotFoundException,
  BadRequestException,
  ConflictException,
  Logger,
} from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { Escrow } from '../../database/entities/escrow.entity';
import { EscrowStatus } from '../../common/enums/escrow-status.enum';
import { Order } from '../../database/entities/order.entity';
import { OrderStatus } from '../../common/enums/order-status.enum';
import {
  CreateEscrowDto,
  UpdateEscrowDto,
  EscrowResponseDto,
  ValidationResultDto,
} from './dto';
import { plainToInstance } from 'class-transformer';

/**
 * Puente lógico orden ↔ contrato
 * 
 * Rol: Puente lógico orden ↔ contrato
 * 
 * Contiene:
 * - Mapeo order_id ↔ escrow_id (bidireccional)
 * - Verificación de consistencia entre orden y escrow
 * - Actualización de estados basada en eventos blockchain
 * 
 * Se conecta con:
 * - blockchain.listener (EventListenerService)
 *   → Recibe eventos: FundsLocked, FundsReleased, FundsRefunded
 *   → Actualiza estados del escrow y orden
 * - orders.service (OrdersService)
 *   → Consulta mapeos: getMapping(), findByOrderId()
 *   → Verifica existencia: existsForOrder(), getStatusForOrder()
 * 
 * ⚠️ REGLA FINAL: Este servicio NUNCA ejecuta transacciones blockchain.
 * Solo:
 * - Mapea order_id ↔ escrow_id
 * - Valida consistencia
 * - Actualiza estados basados en eventos
 * 
 * Las transacciones blockchain son ejecutadas por los usuarios desde el frontend.
 * Este servicio solo reacciona a los eventos emitidos.
 */
@Injectable()
export class EscrowService {
  private readonly logger = new Logger(EscrowService.name);

  constructor(
    @InjectRepository(Escrow)
    private readonly escrowRepository: Repository<Escrow>,
    @InjectRepository(Order)
    private readonly orderRepository: Repository<Order>,
  ) {}

  /**
   * Crea el mapeo order_id ↔ escrow_id
   * 
   * ⚠️ REGLA FINAL: Este método NO ejecuta transacciones blockchain.
   * Solo registra el mapeo después de que el usuario haya creado el escrow
   * en blockchain desde el frontend.
   * 
   * El backend NUNCA debe mover fondos.
   */
  async create(createEscrowDto: CreateEscrowDto): Promise<EscrowResponseDto> {
    const {
      orderId,
      escrowId,
      contractAddress,
      cryptoAmount,
      cryptoCurrency,
      createTransactionHash,
    } = createEscrowDto;

    // Verificar que la orden existe
    const order = await this.orderRepository.findOne({
      where: { id: orderId },
    });

    if (!order) {
      throw new NotFoundException('Orden no encontrada');
    }

    // Verificar que no existe ya un escrow para esta orden
    const existingEscrow = await this.escrowRepository.findOne({
      where: { orderId },
    });

    if (existingEscrow) {
      throw new ConflictException('Ya existe un escrow para esta orden');
    }

    // Verificar que el escrow_id no está en uso
    const existingEscrowId = await this.escrowRepository.findOne({
      where: { escrowId },
    });

    if (existingEscrowId) {
      throw new ConflictException('El escrow_id ya está en uso');
    }

    // Validar consistencia con la orden
    const validation = await this.validateConsistency(orderId, {
      cryptoAmount,
      cryptoCurrency: cryptoCurrency.toUpperCase(),
    });

    if (!validation.isValid) {
      throw new BadRequestException(
        `Inconsistencias detectadas: ${validation.errors.join(', ')}`,
      );
    }

    // Crear el escrow
    const escrow = this.escrowRepository.create({
      orderId,
      escrowId,
      contractAddress,
      cryptoAmount,
      cryptoCurrency: cryptoCurrency.toUpperCase(),
      createTransactionHash,
      status: EscrowStatus.PENDING,
    });

    const savedEscrow = await this.escrowRepository.save(escrow);

    // Actualizar la orden con el escrow_id
    order.escrowId = escrowId;
    await this.orderRepository.save(order);

    this.logger.log(
      `Escrow mapping created: order ${orderId} ↔ escrow ${escrowId}`,
    );

    return this.toResponseDto(savedEscrow);
  }

  /**
   * Valida la consistencia entre una orden y su escrow
   * 
   * Verifica:
   * - Cantidad de criptomoneda
   * - Tipo de criptomoneda
   * - Estados sincronizados
   */
  async validateConsistency(
    orderId: string,
    escrowData?: { cryptoAmount: number; cryptoCurrency: string },
  ): Promise<ValidationResultDto> {
    const order = await this.orderRepository.findOne({
      where: { id: orderId },
    });

    if (!order) {
      return {
        isValid: false,
        errors: ['Orden no encontrada'],
        warnings: [],
        orderId,
        escrowId: null,
      };
    }

    const escrow = await this.escrowRepository.findOne({
      where: { orderId },
    });

    const errors: string[] = [];
    const warnings: string[] = [];

    // Si hay datos de escrow, validar contra la orden
    if (escrowData) {
      // Validar cantidad
      const amountDiff = Math.abs(
        Number(order.cryptoAmount) - escrowData.cryptoAmount,
      );
      if (amountDiff > 0.00000001) {
        errors.push(
          `Cantidad inconsistente: orden=${order.cryptoAmount}, escrow=${escrowData.cryptoAmount}`,
        );
      }

      // Validar moneda
      if (
        order.cryptoCurrency.toUpperCase() !==
        escrowData.cryptoCurrency.toUpperCase()
      ) {
        errors.push(
          `Moneda inconsistente: orden=${order.cryptoCurrency}, escrow=${escrowData.cryptoCurrency}`,
        );
      }
    }

    // Si existe escrow, validar consistencia completa
    if (escrow) {
      // Validar cantidad
      const amountDiff = Math.abs(
        Number(order.cryptoAmount) - Number(escrow.cryptoAmount),
      );
      if (amountDiff > 0.00000001) {
        errors.push(
          `Cantidad inconsistente: orden=${order.cryptoAmount}, escrow=${escrow.cryptoAmount}`,
        );
      }

      // Validar moneda
      if (
        order.cryptoCurrency.toUpperCase() !== escrow.cryptoCurrency.toUpperCase()
      ) {
        errors.push(
          `Moneda inconsistente: orden=${order.cryptoCurrency}, escrow=${escrow.cryptoCurrency}`,
        );
      }

      // Validar estados
      if (
        order.status === OrderStatus.ONCHAIN_LOCKED &&
        escrow.status !== EscrowStatus.LOCKED
      ) {
        warnings.push(
          `Estado inconsistente: orden=${order.status}, escrow=${escrow.status}`,
        );
      }

      if (
        order.status === OrderStatus.COMPLETED &&
        escrow.status !== EscrowStatus.RELEASED
      ) {
        warnings.push(
          `Estado inconsistente: orden=${order.status}, escrow=${escrow.status}`,
        );
      }

      if (
        order.status === OrderStatus.REFUNDED &&
        escrow.status !== EscrowStatus.REFUNDED
      ) {
        warnings.push(
          `Estado inconsistente: orden=${order.status}, escrow=${escrow.status}`,
        );
      }
    } else if (order.escrowId) {
      errors.push(
        'La orden tiene escrowId pero no existe registro de escrow',
      );
    }

    // Guardar errores en el escrow si existe
    if (escrow && errors.length > 0) {
      escrow.validationErrors = errors.join('; ');
      await this.escrowRepository.save(escrow);
    }

    return {
      isValid: errors.length === 0,
      errors,
      warnings,
      orderId,
      escrowId: escrow?.escrowId || null,
    };
  }

  /**
   * Obtiene el escrow por ID
   */
  async findOne(id: string): Promise<EscrowResponseDto> {
    const escrow = await this.escrowRepository.findOne({
      where: { id },
    });

    if (!escrow) {
      throw new NotFoundException('Escrow no encontrado');
    }

    return this.toResponseDto(escrow);
  }

  /**
   * Obtiene el escrow por order_id
   * 
   * Usado por orders.service para consultar el escrow de una orden
   */
  async findByOrderId(orderId: string): Promise<EscrowResponseDto> {
    const escrow = await this.escrowRepository.findOne({
      where: { orderId },
    });

    if (!escrow) {
      throw new NotFoundException('Escrow no encontrado para esta orden');
    }

    return this.toResponseDto(escrow);
  }

  /**
   * Obtiene el escrow por escrow_id (blockchain)
   * 
   * Usado por blockchain.listener para encontrar el escrow desde eventos
   */
  async findByEscrowId(escrowId: string): Promise<EscrowResponseDto> {
    const escrow = await this.escrowRepository.findOne({
      where: { escrowId },
    });

    if (!escrow) {
      throw new NotFoundException('Escrow no encontrado');
    }

    return this.toResponseDto(escrow);
  }

  /**
   * Actualiza el estado del escrow basado en eventos de blockchain
   * 
   * ⚠️ REGLA FINAL: Este método NO ejecuta transacciones.
   * Solo actualiza el estado off-chain basado en eventos de blockchain.
   * 
   * Usado por blockchain.listener cuando recibe eventos:
   * - FundsLocked → status = LOCKED
   * - FundsReleased → status = RELEASED
   * - FundsRefunded → status = REFUNDED
   */
  async update(
    escrowId: string,
    updateEscrowDto: UpdateEscrowDto,
  ): Promise<EscrowResponseDto> {
    const escrow = await this.escrowRepository.findOne({
      where: { escrowId },
    });

    if (!escrow) {
      throw new NotFoundException('Escrow no encontrado');
    }

    const { status, releaseTransactionHash, refundTransactionHash } =
      updateEscrowDto;

    // Actualizar estado
    if (status) {
      escrow.status = status;

      // Actualizar timestamps según el estado
      if (status === EscrowStatus.LOCKED && !escrow.lockedAt) {
        escrow.lockedAt = new Date();
        // Actualizar estado de la orden directamente (evita dependencia circular)
        await this.orderRepository.update(
          { id: escrow.orderId },
          { status: OrderStatus.ONCHAIN_LOCKED },
        );
      } else if (status === EscrowStatus.RELEASED && !escrow.releasedAt) {
        escrow.releasedAt = new Date();
        // Actualizar estado de la orden
        await this.orderRepository.update(
          { id: escrow.orderId },
          { status: OrderStatus.COMPLETED, completedAt: new Date() },
        );
      } else if (status === EscrowStatus.REFUNDED && !escrow.refundedAt) {
        escrow.refundedAt = new Date();
        // Actualizar estado de la orden
        await this.orderRepository.update(
          { id: escrow.orderId },
          { status: OrderStatus.REFUNDED },
        );
      }
    }

    // Actualizar hashes de transacciones
    if (releaseTransactionHash) {
      escrow.releaseTransactionHash = releaseTransactionHash;
    }
    if (refundTransactionHash) {
      escrow.refundTransactionHash = refundTransactionHash;
    }

    // Limpiar errores de validación si se actualiza
    if (status && escrow.validationErrors) {
      escrow.validationErrors = null;
    }

    const savedEscrow = await this.escrowRepository.save(escrow);

    // Validar consistencia después de actualizar
    await this.validateConsistency(escrow.orderId);

    this.logger.debug(`Escrow updated: ${escrowId} -> ${status || 'no status change'}`);

    return this.toResponseDto(savedEscrow);
  }

  /**
   * Maneja evento EscrowCreated desde blockchain.listener
   * 
   * Se conecta con: blockchain.listener (EventListenerService)
   * 
   * Este método puede ser llamado cuando se detecta un evento EscrowCreated
   * para crear el mapeo si aún no existe.
   */
  async handleEscrowCreated(
    escrowId: string,
    orderId: string,
    transactionHash: string,
    contractAddress: string,
    cryptoAmount: number,
    cryptoCurrency: string,
  ): Promise<EscrowResponseDto> {
    this.logger.log(`Handling EscrowCreated event for escrow ${escrowId}, order ${orderId}`);
    
    // Verificar si ya existe el mapeo
    const existing = await this.escrowRepository.findOne({
      where: { escrowId },
    });

    if (existing) {
      this.logger.debug(`Escrow mapping already exists for ${escrowId}`);
      return this.toResponseDto(existing);
    }

    // Crear el mapeo
    return this.create({
      orderId,
      escrowId,
      contractAddress,
      cryptoAmount,
      cryptoCurrency,
      createTransactionHash: transactionHash,
    });
  }

  /**
   * Maneja evento FundsLocked desde blockchain.listener
   * 
   * Se conecta con: blockchain.listener (EventListenerService)
   * 
   * Cuando se detecta que los fondos fueron bloqueados en blockchain,
   * actualiza el estado del escrow y la orden.
   */
  async handleFundsLocked(
    escrowId: string,
    transactionHash: string,
  ): Promise<EscrowResponseDto> {
    this.logger.log(`Handling FundsLocked event for escrow ${escrowId}`);
    
    return this.update(escrowId, {
      status: EscrowStatus.LOCKED,
    });
  }

  /**
   * Maneja evento FundsReleased desde blockchain.listener
   * 
   * Se conecta con: blockchain.listener (EventListenerService)
   * 
   * Cuando se detecta que los fondos fueron liberados en blockchain,
   * actualiza el estado del escrow y marca la orden como completada.
   */
  async handleFundsReleased(
    escrowId: string,
    transactionHash: string,
  ): Promise<EscrowResponseDto> {
    this.logger.log(`Handling FundsReleased event for escrow ${escrowId}`);
    
    return this.update(escrowId, {
      status: EscrowStatus.RELEASED,
      releaseTransactionHash: transactionHash,
    });
  }

  /**
   * Maneja evento FundsRefunded desde blockchain.listener
   * 
   * Se conecta con: blockchain.listener (EventListenerService)
   * 
   * Cuando se detecta que los fondos fueron reembolsados en blockchain,
   * actualiza el estado del escrow y marca la orden como reembolsada.
   */
  async handleFundsRefunded(
    escrowId: string,
    transactionHash: string,
  ): Promise<EscrowResponseDto> {
    this.logger.log(`Handling FundsRefunded event for escrow ${escrowId}`);
    
    return this.update(escrowId, {
      status: EscrowStatus.REFUNDED,
      refundTransactionHash: transactionHash,
    });
  }

  /**
   * Obtiene el mapeo order_id ↔ escrow_id
   * 
   * Usado por orders.service para consultar el mapeo
   */
  async getMapping(orderId?: string, escrowId?: string): Promise<{
    orderId: string;
    escrowId: string;
  }> {
    if (orderId) {
      const escrow = await this.escrowRepository.findOne({
        where: { orderId },
      });
      if (!escrow) {
        throw new NotFoundException('Mapeo no encontrado para esta orden');
      }
      return {
        orderId: escrow.orderId,
        escrowId: escrow.escrowId,
      };
    }

    if (escrowId) {
      const escrow = await this.escrowRepository.findOne({
        where: { escrowId },
      });
      if (!escrow) {
        throw new NotFoundException('Mapeo no encontrado para este escrow');
      }
      return {
        orderId: escrow.orderId,
        escrowId: escrow.escrowId,
      };
    }

    throw new BadRequestException('Debe proporcionar orderId o escrowId');
  }

  /**
   * Lista todos los escrows con filtros
   */
  async findAll(
    orderId?: string,
    escrowId?: string,
    status?: EscrowStatus,
  ): Promise<EscrowResponseDto[]> {
    const where: any = {};

    if (orderId) {
      where.orderId = orderId;
    }
    if (escrowId) {
      where.escrowId = escrowId;
    }
    if (status) {
      where.status = status;
    }

    const escrows = await this.escrowRepository.find({
      where,
      order: {
        createdAt: 'DESC',
      },
    });

    return escrows.map((escrow) => this.toResponseDto(escrow));
  }

  /**
   * Verifica si existe un escrow para una orden
   * 
   * Usado por orders.service para verificar si una orden tiene escrow
   */
  async existsForOrder(orderId: string): Promise<boolean> {
    const count = await this.escrowRepository.count({
      where: { orderId },
    });
    return count > 0;
  }

  /**
   * Obtiene el estado del escrow para una orden
   * 
   * Usado por orders.service para obtener el estado del escrow
   */
  async getStatusForOrder(orderId: string): Promise<EscrowStatus | null> {
    const escrow = await this.escrowRepository.findOne({
      where: { orderId },
      select: ['status'],
    });
    return escrow?.status || null;
  }

  /**
   * Convierte Escrow a DTO de respuesta
   */
  private toResponseDto(escrow: Escrow): EscrowResponseDto {
    return plainToInstance(EscrowResponseDto, {
      id: escrow.id,
      order_id: escrow.orderId,
      escrow_id: escrow.escrowId,
      contract_address: escrow.contractAddress,
      create_transaction_hash: escrow.createTransactionHash,
      crypto_amount: Number(escrow.cryptoAmount),
      crypto_currency: escrow.cryptoCurrency,
      status: escrow.status,
      release_transaction_hash: escrow.releaseTransactionHash,
      refund_transaction_hash: escrow.refundTransactionHash,
      locked_at: escrow.lockedAt,
      released_at: escrow.releasedAt,
      refunded_at: escrow.refundedAt,
      validation_errors: escrow.validationErrors,
      created_at: escrow.createdAt,
      updated_at: escrow.updatedAt,
    });
  }
}
