import {
  Injectable,
  NotFoundException,
  BadRequestException,
  ForbiddenException,
  Logger,
} from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository, FindOptionsWhere } from 'typeorm';
import { Dispute } from '../../database/entities/dispute.entity';
import { DisputeEvidence } from '../../database/entities/dispute-evidence.entity';
import { Order } from '../../database/entities/order.entity';
import { DisputeStatus } from '../../common/enums/dispute-status.enum';
import { OrderStatus } from '../../common/enums/order-status.enum';
import { CreateDisputeDto, AddEvidenceDto, ResolveDisputeDto } from './dto';
import { ReputationService } from '../reputation/reputation.service';
import { EscrowService } from '../escrow/escrow.service';
import { EscrowStatus } from '../../common/enums/escrow-status.enum';

@Injectable()
export class DisputesService {
  private readonly logger = new Logger(DisputesService.name);
  private readonly TIMERS = {
    RESPONSE_DEADLINE_HOURS: 48, // 48 horas para responder
    EVIDENCE_DEADLINE_HOURS: 72, // 72 horas para presentar evidencia
    ESCALATION_DAYS: 7, // 7 días antes de escalar
  };

  constructor(
    @InjectRepository(Dispute)
    private readonly disputeRepository: Repository<Dispute>,
    @InjectRepository(DisputeEvidence)
    private readonly evidenceRepository: Repository<DisputeEvidence>,
    @InjectRepository(Order)
    private readonly orderRepository: Repository<Order>,
    private readonly reputationService: ReputationService,
    private readonly escrowService: EscrowService,
  ) {}

  /**
   * Abre una nueva disputa
   */
  async create(
    userId: string,
    createDisputeDto: CreateDisputeDto,
  ): Promise<Dispute> {
    const { orderId, reason } = createDisputeDto;

    // Verificar que la orden existe
    const order = await this.orderRepository.findOne({
      where: { id: orderId },
    });

    if (!order) {
      throw new NotFoundException('Orden no encontrada');
    }

    // Verificar que el usuario es parte de la orden
    if (order.sellerId !== userId && order.buyerId !== userId) {
      throw new ForbiddenException('No eres parte de esta orden');
    }

    // Verificar que la orden puede ser disputada
    const disputableStatuses = [
      OrderStatus.AWAITING_FUNDS,
      OrderStatus.ONCHAIN_LOCKED,
    ];

    if (!disputableStatuses.includes(order.status)) {
      throw new BadRequestException(
        `No se puede abrir una disputa para una orden en estado ${order.status}`,
      );
    }

    // Verificar que no existe ya una disputa abierta
    const existingDispute = await this.disputeRepository.findOne({
      where: {
        orderId,
        status: DisputeStatus.OPEN,
      },
    });

    if (existingDispute) {
      throw new BadRequestException('Ya existe una disputa abierta para esta orden');
    }

    // Determinar el respondent (la otra parte)
    const respondentId =
      order.sellerId === userId ? order.buyerId : order.sellerId;

    // Calcular deadlines
    const now = new Date();
    const responseDeadline = new Date(
      now.getTime() + this.TIMERS.RESPONSE_DEADLINE_HOURS * 60 * 60 * 1000,
    );
    const evidenceDeadline = new Date(
      now.getTime() + this.TIMERS.EVIDENCE_DEADLINE_HOURS * 60 * 60 * 1000,
    );
    const expiresAt = new Date(
      now.getTime() + this.TIMERS.ESCALATION_DAYS * 24 * 60 * 60 * 1000,
    );

    // Crear disputa
    const dispute = this.disputeRepository.create({
      orderId,
      initiatorId: userId,
      respondentId,
      reason,
      status: DisputeStatus.OPEN,
      responseDeadline,
      evidenceDeadline,
      expiresAt,
    });

    const savedDispute = await this.disputeRepository.save(dispute);

    // Actualizar estado de la orden
    order.status = OrderStatus.DISPUTED;
    await this.orderRepository.save(order);

    // Registrar evento de reputation (penalización por abrir disputa)
    await this.reputationService.recordDisputeOpened(
      userId,
      orderId,
      savedDispute.id,
    );

    // Si hay respondent, también penalizar
    if (respondentId) {
      await this.reputationService.recordDisputeOpened(
        respondentId,
        orderId,
        savedDispute.id,
      );
    }

    // Marcar escrow como disputado
    if (order.escrowId) {
      try {
        await this.escrowService.update(order.escrowId, {
          status: EscrowStatus.DISPUTED,
        });
      } catch (error) {
        this.logger.warn(`Could not update escrow status: ${error.message}`);
      }
    }

    this.logger.log(
      `Dispute created: ${savedDispute.id} for order ${orderId} by ${userId}`,
    );

    return savedDispute;
  }

  /**
   * Agrega evidencia off-chain a una disputa
   */
  async addEvidence(
    disputeId: string,
    userId: string,
    addEvidenceDto: AddEvidenceDto,
  ): Promise<DisputeEvidence> {
    const dispute = await this.disputeRepository.findOne({
      where: { id: disputeId },
      relations: ['evidence'],
    });

    if (!dispute) {
      throw new NotFoundException('Disputa no encontrada');
    }

    // Verificar que el usuario es parte de la disputa
    if (dispute.initiatorId !== userId && dispute.respondentId !== userId) {
      throw new ForbiddenException('No eres parte de esta disputa');
    }

    // Verificar que la disputa está abierta
    if (dispute.status !== DisputeStatus.OPEN && dispute.status !== DisputeStatus.IN_REVIEW) {
      throw new BadRequestException(
        `No se puede agregar evidencia a una disputa en estado ${dispute.status}`,
      );
    }

    // Verificar deadline de evidencia
    if (dispute.evidenceDeadline && new Date(dispute.evidenceDeadline) < new Date()) {
      throw new BadRequestException('El plazo para presentar evidencia ha expirado');
    }

    // Crear evidencia
    const evidence = this.evidenceRepository.create({
      disputeId,
      submittedById: userId,
      evidenceType: addEvidenceDto.evidenceType,
      evidenceUrl: addEvidenceDto.evidenceUrl,
      description: addEvidenceDto.description,
    });

    const savedEvidence = await this.evidenceRepository.save(evidence);

    // Actualizar estado a IN_REVIEW si hay evidencia
    if (dispute.status === DisputeStatus.OPEN) {
      dispute.status = DisputeStatus.IN_REVIEW;
      await this.disputeRepository.save(dispute);
    }

    this.logger.log(
      `Evidence added to dispute ${disputeId} by ${userId}`,
    );

    return savedEvidence;
  }

  /**
   * Resuelve una disputa
   * IMPORTANTE: La resolución final siempre depende del escrow
   */
  async resolve(
    disputeId: string,
    resolveDto: ResolveDisputeDto,
  ): Promise<Dispute> {
    const dispute = await this.disputeRepository.findOne({
      where: { id: disputeId },
      relations: ['order'],
    });

    if (!dispute) {
      throw new NotFoundException('Disputa no encontrada');
    }

    // Verificar que la disputa puede ser resuelta
    if (dispute.status === DisputeStatus.RESOLVED || dispute.status === DisputeStatus.CLOSED) {
      throw new BadRequestException('La disputa ya está resuelta o cerrada');
    }

    // IMPORTANTE: La resolución final siempre viene del escrow
    // Este método solo registra la resolución off-chain
    // El escrow es quien decide los fondos

    const { resolution, escrowResolution } = resolveDto;

    // Si hay resolución del escrow, usarla como definitiva
    if (escrowResolution) {
      dispute.escrowResolution = escrowResolution;
      dispute.escrowResolvedAt = new Date();
    }

    dispute.status = DisputeStatus.RESOLVED;
    dispute.resolution = resolution;
    dispute.resolvedAt = new Date();

    const savedDispute = await this.disputeRepository.save(dispute);

    // Actualizar reputation según resolución del escrow
    if (escrowResolution) {
      // Determinar quién ganó basado en la resolución del escrow
      // Esto es un ejemplo, la lógica real depende de cómo el escrow reporte la resolución
      const order = await this.orderRepository.findOne({
        where: { id: dispute.orderId },
      });

      if (order) {
        // Si el escrow resolvió a favor del initiator
        if (escrowResolution.includes('INITIATOR') || escrowResolution.includes('SELLER') && dispute.initiatorId === order.sellerId) {
          await this.reputationService.recordDisputeResolvedFavor(
            dispute.initiatorId,
            dispute.orderId,
            dispute.id,
          );
          if (dispute.respondentId) {
            await this.reputationService.recordDisputeResolvedAgainst(
              dispute.respondentId,
              dispute.orderId,
              dispute.id,
            );
          }
        } else {
          // Resuelto a favor del respondent
          if (dispute.respondentId) {
            await this.reputationService.recordDisputeResolvedFavor(
              dispute.respondentId,
              dispute.orderId,
              dispute.id,
            );
          }
          await this.reputationService.recordDisputeResolvedAgainst(
            dispute.initiatorId,
            dispute.orderId,
            dispute.id,
          );
        }
      }
    }

    this.logger.log(`Dispute resolved: ${disputeId}`);

    return savedDispute;
  }

  /**
   * Cierra una disputa (después de que el escrow resuelve)
   */
  async close(disputeId: string, escrowResolution: string): Promise<Dispute> {
    const dispute = await this.disputeRepository.findOne({
      where: { id: disputeId },
    });

    if (!dispute) {
      throw new NotFoundException('Disputa no encontrada');
    }

    dispute.status = DisputeStatus.CLOSED;
    dispute.escrowResolution = escrowResolution;
    dispute.escrowResolvedAt = new Date();
    dispute.resolvedAt = new Date();

    const savedDispute = await this.disputeRepository.save(dispute);

    this.logger.log(`Dispute closed: ${disputeId} with escrow resolution: ${escrowResolution}`);

    return savedDispute;
  }

  /**
   * Escala una disputa (si expira sin resolución)
   */
  async escalate(disputeId: string): Promise<Dispute> {
    const dispute = await this.disputeRepository.findOne({
      where: { id: disputeId },
    });

    if (!dispute) {
      throw new NotFoundException('Disputa no encontrada');
    }

    dispute.status = DisputeStatus.ESCALATED;
    dispute.escalatedAt = new Date();

    const savedDispute = await this.disputeRepository.save(dispute);

    this.logger.log(`Dispute escalated: ${disputeId}`);

    return savedDispute;
  }

  /**
   * Obtiene una disputa por ID
   */
  async findOne(id: string): Promise<Dispute> {
    const dispute = await this.disputeRepository.findOne({
      where: { id },
      relations: ['evidence', 'order', 'initiator', 'respondent'],
    });

    if (!dispute) {
      throw new NotFoundException('Disputa no encontrada');
    }

    return dispute;
  }

  /**
   * Lista todas las disputas con filtros
   */
  async findAll(
    status?: DisputeStatus,
    orderId?: string,
    userId?: string,
  ): Promise<Dispute[]> {
    const where: FindOptionsWhere<Dispute> = {};

    if (status) {
      where.status = status;
    }
    if (orderId) {
      where.orderId = orderId;
    }
    if (userId) {
      where.initiatorId = userId;
    }

    return this.disputeRepository.find({
      where,
      relations: ['evidence', 'order'],
      order: { createdAt: 'DESC' },
    });
  }

  /**
   * Verifica y procesa timers de disputas
   */
  async processTimers(): Promise<{
    escalated: number;
    expired: number;
  }> {
    const now = new Date();
    let escalated = 0;
    let expired = 0;

    // Disputas que deben escalarse
    const disputesToEscalate = await this.disputeRepository.find({
      where: [
        { status: DisputeStatus.OPEN },
        { status: DisputeStatus.IN_REVIEW },
      ],
    });

    for (const dispute of disputesToEscalate) {
      // Verificar si expiró
      if (dispute.expiresAt && new Date(dispute.expiresAt) < now) {
        await this.escalate(dispute.id);
        escalated++;
      } else if (dispute.evidenceDeadline && new Date(dispute.evidenceDeadline) < now) {
        // Marcar como expirada si pasó el deadline de evidencia
        expired++;
      }
    }

    return { escalated, expired };
  }

  /**
   * Obtiene disputas expiradas o próximas a expirar
   */
  async getExpiringDisputes(hours: number = 24): Promise<Dispute[]> {
    const deadline = new Date(Date.now() + hours * 60 * 60 * 1000);

    return this.disputeRepository
      .createQueryBuilder('dispute')
      .where('dispute.status IN (:...statuses)', {
        statuses: [DisputeStatus.OPEN, DisputeStatus.IN_REVIEW],
      })
      .andWhere('dispute.expires_at <= :deadline', { deadline })
      .andWhere('dispute.expires_at > :now', { now: new Date() })
      .getMany();
  }
}