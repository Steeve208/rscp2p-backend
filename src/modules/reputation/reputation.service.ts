import {
  Injectable,
  NotFoundException,
  BadRequestException,
  Logger,
} from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { User } from '../../database/entities/user.entity';
import { ReputationEvent, ReputationEventType } from '../../database/entities/reputation-event.entity';
import { Order } from '../../database/entities/order.entity';
import { OrderStatus } from '../../common/enums/order-status.enum';

/**
 * Servicio de cálculo de confianza (Reputation)
 * 
 * Rol: Cálculo de confianza
 * 
 * Contiene:
 * - Algoritmo de score avanzado
 * - Cálculo de reputation basado en eventos
 * - Decaimiento temporal
 * - Factores de volumen y tasa de éxito
 * 
 * Nunca debe:
 * - Afectar fondos
 * - Decidir sobre transacciones blockchain
 * - Ejecutar transacciones
 * - Mover tokens o criptomonedas
 * 
 * ⚠️ REGLA FINAL: Este servicio SOLO calcula scores de confianza.
 * NUNCA debe afectar fondos o ejecutar transacciones blockchain.
 * 
 * El score es puramente informativo y ayuda a los usuarios
 * a tomar decisiones sobre con quién hacer trades.
 */
@Injectable()
export class ReputationService {
  private readonly logger = new Logger(ReputationService.name);

  // Configuración de scoring base
  private readonly SCORING = {
    TRADE_COMPLETED: 5, // +5 por trade completado
    TRADE_CANCELLED: -10, // -10 por cancelar trade
    DISPUTE_OPENED: -15, // -15 por abrir disputa
    DISPUTE_RESOLVED_FAVOR: 10, // +10 si gana disputa
    DISPUTE_RESOLVED_AGAINST: -20, // -20 si pierde disputa
    PENALTY: -25, // -25 por penalización manual
    BONUS: 15, // +15 por bonus manual
  };

  // Configuración del algoritmo avanzado
  private readonly ALGORITHM = {
    MIN_SCORE: -100, // Score mínimo
    MAX_SCORE: 100, // Score máximo
    DECAY_DAYS: 90, // Días para decaimiento temporal (eventos más antiguos pesan menos)
    VOLUME_BONUS_THRESHOLD: 10, // Número de trades para activar bonus de volumen
    VOLUME_BONUS_MULTIPLIER: 1.1, // Multiplicador por volumen alto
    SUCCESS_RATE_BONUS_THRESHOLD: 0.8, // 80% de tasa de éxito para bonus
    SUCCESS_RATE_BONUS: 5, // Bonus adicional por alta tasa de éxito
    REPEAT_OFFENDER_PENALTY: 1.5, // Multiplicador de penalización por comportamiento repetitivo
  };

  constructor(
    @InjectRepository(User)
    private readonly userRepository: Repository<User>,
    @InjectRepository(ReputationEvent)
    private readonly eventRepository: Repository<ReputationEvent>,
    @InjectRepository(Order)
    private readonly orderRepository: Repository<Order>,
  ) {}

  /**
   * Obtiene la reputation de un usuario
   */
  async getUserReputation(userId: string): Promise<{
    userId: string;
    walletAddress: string;
    score: number;
    totalTrades: number;
    completedTrades: number;
    cancelledTrades: number;
    disputesOpened: number;
    disputesWon: number;
    disputesLost: number;
    recentEvents: ReputationEvent[];
  }> {
    const user = await this.userRepository.findOne({
      where: { id: userId },
    });

    if (!user) {
      throw new NotFoundException('Usuario no encontrado');
    }

    // Obtener estadísticas de trades
    const totalTrades = await this.orderRepository.count({
      where: [
        { sellerId: userId },
        { buyerId: userId },
      ],
    });

    const completedTrades = await this.orderRepository.count({
      where: [
        { sellerId: userId, status: OrderStatus.COMPLETED },
        { buyerId: userId, status: OrderStatus.COMPLETED },
      ],
    });

    const cancelledTrades = await this.orderRepository.count({
      where: [
        { sellerId: userId, status: OrderStatus.REFUNDED },
        { buyerId: userId, status: OrderStatus.REFUNDED },
      ],
    });

    // Obtener estadísticas de disputas
    const disputesOpened = await this.eventRepository.count({
      where: {
        userId,
        eventType: ReputationEventType.DISPUTE_OPENED,
      },
    });

    const disputesWon = await this.eventRepository.count({
      where: {
        userId,
        eventType: ReputationEventType.DISPUTE_RESOLVED_FAVOR,
      },
    });

    const disputesLost = await this.eventRepository.count({
      where: {
        userId,
        eventType: ReputationEventType.DISPUTE_RESOLVED_AGAINST,
      },
    });

    // Obtener eventos recientes
    const recentEvents = await this.eventRepository.find({
      where: { userId },
      order: { createdAt: 'DESC' },
      take: 10,
    });

    return {
      userId: user.id,
      walletAddress: user.walletAddress,
      score: Number(user.reputationScore),
      totalTrades,
      completedTrades,
      cancelledTrades,
      disputesOpened,
      disputesWon,
      disputesLost,
      recentEvents,
    };
  }

  /**
   * Registra un evento de trade completado
   * NO decide fondos, solo actualiza reputation
   */
  async recordTradeCompleted(
    userId: string,
    orderId: string,
  ): Promise<ReputationEvent> {
    return this.recordEvent(
      userId,
      ReputationEventType.TRADE_COMPLETED,
      this.SCORING.TRADE_COMPLETED,
      orderId,
      null,
      'Trade completado exitosamente',
    );
  }

  /**
   * Registra un evento de trade cancelado
   */
  async recordTradeCancelled(
    userId: string,
    orderId: string,
    reason?: string,
  ): Promise<ReputationEvent> {
    return this.recordEvent(
      userId,
      ReputationEventType.TRADE_CANCELLED,
      this.SCORING.TRADE_CANCELLED,
      orderId,
      null,
      reason || 'Trade cancelado',
    );
  }

  /**
   * Registra un evento de disputa abierta
   */
  async recordDisputeOpened(
    userId: string,
    orderId: string,
    disputeId: string,
  ): Promise<ReputationEvent> {
    return this.recordEvent(
      userId,
      ReputationEventType.DISPUTE_OPENED,
      this.SCORING.DISPUTE_OPENED,
      orderId,
      disputeId,
      'Disputa abierta',
    );
  }

  /**
   * Registra resolución de disputa a favor del usuario
   */
  async recordDisputeResolvedFavor(
    userId: string,
    orderId: string,
    disputeId: string,
  ): Promise<ReputationEvent> {
    return this.recordEvent(
      userId,
      ReputationEventType.DISPUTE_RESOLVED_FAVOR,
      this.SCORING.DISPUTE_RESOLVED_FAVOR,
      orderId,
      disputeId,
      'Disputa resuelta a favor',
    );
  }

  /**
   * Registra resolución de disputa en contra del usuario
   */
  async recordDisputeResolvedAgainst(
    userId: string,
    orderId: string,
    disputeId: string,
  ): Promise<ReputationEvent> {
    return this.recordEvent(
      userId,
      ReputationEventType.DISPUTE_RESOLVED_AGAINST,
      this.SCORING.DISPUTE_RESOLVED_AGAINST,
      orderId,
      disputeId,
      'Disputa resuelta en contra',
    );
  }

  /**
   * Aplica una penalización manual
   */
  async applyPenalty(
    userId: string,
    reason: string,
    orderId?: string,
    disputeId?: string,
  ): Promise<ReputationEvent> {
    return this.recordEvent(
      userId,
      ReputationEventType.PENALTY,
      this.SCORING.PENALTY,
      orderId,
      disputeId,
      reason,
    );
  }

  /**
   * Aplica un bonus manual
   */
  async applyBonus(
    userId: string,
    reason: string,
    orderId?: string,
    disputeId?: string,
  ): Promise<ReputationEvent> {
    return this.recordEvent(
      userId,
      ReputationEventType.BONUS,
      this.SCORING.BONUS,
      orderId,
      disputeId,
      reason,
    );
  }

  /**
   * Calcula el score usando el algoritmo avanzado
   * 
   * Considera:
   * - Decaimiento temporal (eventos recientes pesan más)
   * - Volumen de trades
   * - Tasa de éxito
   * - Comportamiento repetitivo
   * 
   * ⚠️ REGLA FINAL: Este método SOLO calcula scores.
   * NUNCA afecta fondos o ejecuta transacciones.
   */
  private calculateAdvancedScore(
    events: ReputationEvent[],
    currentBaseScore: number,
  ): number {
    if (events.length === 0) {
      return currentBaseScore;
    }

    const now = new Date();
    let adjustedScore = 0;

    // Contadores para análisis
    let completedTrades = 0;
    let cancelledTrades = 0;
    let disputesOpened = 0;
    let recentNegativeEvents = 0; // Eventos negativos en los últimos 30 días

    // Calcular score con decaimiento temporal
    events.forEach((event) => {
      const daysAgo = Math.floor(
        (now.getTime() - event.createdAt.getTime()) / (1000 * 60 * 60 * 24),
      );

      // Factor de decaimiento temporal (eventos más recientes pesan más)
      const decayFactor = Math.max(
        0.5,
        1 - daysAgo / this.ALGORITHM.DECAY_DAYS,
      );

      const adjustedChange = Number(event.scoreChange) * decayFactor;
      adjustedScore += adjustedChange;

      // Contar eventos para análisis
      if (event.eventType === ReputationEventType.TRADE_COMPLETED) {
        completedTrades++;
      } else if (event.eventType === ReputationEventType.TRADE_CANCELLED) {
        cancelledTrades++;
      } else if (event.eventType === ReputationEventType.DISPUTE_OPENED) {
        disputesOpened++;
      }

      // Contar eventos negativos recientes
      if (
        Number(event.scoreChange) < 0 &&
        daysAgo <= 30
      ) {
        recentNegativeEvents++;
      }
    });

    // Bonus por volumen alto
    const totalTrades = completedTrades + cancelledTrades;
    if (totalTrades >= this.ALGORITHM.VOLUME_BONUS_THRESHOLD) {
      adjustedScore *= this.ALGORITHM.VOLUME_BONUS_MULTIPLIER;
    }

    // Bonus por alta tasa de éxito
    if (totalTrades > 0) {
      const successRate = completedTrades / totalTrades;
      if (successRate >= this.ALGORITHM.SUCCESS_RATE_BONUS_THRESHOLD) {
        adjustedScore += this.ALGORITHM.SUCCESS_RATE_BONUS;
      }
    }

    // Penalización por comportamiento repetitivo (muchos eventos negativos recientes)
    if (recentNegativeEvents >= 3) {
      adjustedScore *= this.ALGORITHM.REPEAT_OFFENDER_PENALTY;
    }

    // Aplicar límites
    adjustedScore = Math.max(
      this.ALGORITHM.MIN_SCORE,
      Math.min(this.ALGORITHM.MAX_SCORE, adjustedScore),
    );

    return Math.round(adjustedScore * 100) / 100; // Redondear a 2 decimales
  }

  /**
   * Registra un evento de reputation
   * 
   * ⚠️ REGLA FINAL: NO decide fondos, solo actualiza el score.
   * Este método NUNCA afecta fondos o ejecuta transacciones blockchain.
   */
  private async recordEvent(
    userId: string,
    eventType: ReputationEventType,
    scoreChange: number,
    orderId: string | null,
    disputeId: string | null,
    reason: string,
    metadata?: any,
  ): Promise<ReputationEvent> {
    const user = await this.userRepository.findOne({
      where: { id: userId },
    });

    if (!user) {
      throw new NotFoundException('Usuario no encontrado');
    }

    const previousScore = Number(user.reputationScore);

    // Obtener todos los eventos para calcular score avanzado
    const allEvents = await this.eventRepository.find({
      where: { userId },
      order: { createdAt: 'ASC' },
    });

    // Calcular nuevo score usando algoritmo avanzado
    // Primero agregamos el evento actual a la lista temporalmente
    const tempEvent: Partial<ReputationEvent> = {
      eventType,
      scoreChange,
      createdAt: new Date(),
    };
    const eventsWithNew = [...allEvents, tempEvent as ReputationEvent];

    const newScore = this.calculateAdvancedScore(
      eventsWithNew,
      previousScore,
    );

    // Asegurar que el cambio no exceda los límites
    const finalScore = Math.max(
      this.ALGORITHM.MIN_SCORE,
      Math.min(this.ALGORITHM.MAX_SCORE, newScore),
    );

    // Crear evento
    const event = this.eventRepository.create({
      userId,
      eventType,
      scoreChange,
      orderId,
      disputeId,
      reason,
      metadata: metadata ? JSON.stringify(metadata) : null,
      previousScore,
      newScore: finalScore,
    });

    await this.eventRepository.save(event);

    // Actualizar score del usuario
    // ⚠️ IMPORTANTE: Esto solo actualiza un campo en la base de datos.
    // NO afecta fondos, NO ejecuta transacciones blockchain.
    user.reputationScore = finalScore;
    await this.userRepository.save(user);

    this.logger.log(
      `Reputation event recorded: ${eventType} for user ${userId}. Score: ${previousScore} → ${finalScore}`,
    );

    return event;
  }

  /**
   * Obtiene el historial de eventos de reputation
   */
  async getReputationHistory(
    userId: string,
    limit: number = 50,
  ): Promise<ReputationEvent[]> {
    return this.eventRepository.find({
      where: { userId },
      order: { createdAt: 'DESC' },
      take: limit,
    });
  }

  /**
   * Obtiene estadísticas de reputation
   */
  async getReputationStats(userId: string): Promise<{
    totalEvents: number;
    eventsByType: Record<string, number>;
    totalScoreChange: number;
    averageScoreChange: number;
  }> {
    const events = await this.eventRepository.find({
      where: { userId },
    });

    const eventsByType: Record<string, number> = {};
    let totalScoreChange = 0;

    events.forEach((event) => {
      eventsByType[event.eventType] = (eventsByType[event.eventType] || 0) + 1;
      totalScoreChange += Number(event.scoreChange);
    });

    return {
      totalEvents: events.length,
      eventsByType,
      totalScoreChange,
      averageScoreChange:
        events.length > 0 ? totalScoreChange / events.length : 0,
    };
  }

  /**
   * Calcula reputation basado en eventos históricos usando el algoritmo avanzado
   * Útil para re-calcular si cambian las reglas
   * 
   * ⚠️ REGLA FINAL: Este método SOLO recalcula scores.
   * NUNCA afecta fondos o ejecuta transacciones blockchain.
   */
  async recalculateReputation(userId: string): Promise<{
    calculatedScore: number;
    currentScore: number;
    difference: number;
    algorithm: 'advanced';
  }> {
    const user = await this.userRepository.findOne({
      where: { id: userId },
    });

    if (!user) {
      throw new NotFoundException('Usuario no encontrado');
    }

    const events = await this.eventRepository.find({
      where: { userId },
      order: { createdAt: 'ASC' },
    });

    // Calcular score usando algoritmo avanzado
    const calculatedScore = this.calculateAdvancedScore(events, 0);
    const currentScore = Number(user.reputationScore);
    const difference = calculatedScore - currentScore;

    return {
      calculatedScore,
      currentScore,
      difference,
      algorithm: 'advanced',
    };
  }

  /**
   * Aplica el score calculado al usuario
   * 
   * ⚠️ REGLA FINAL: Este método SOLO actualiza el campo reputationScore.
   * NUNCA afecta fondos o ejecuta transacciones blockchain.
   */
  async applyCalculatedScore(
    userId: string,
    newScore: number,
  ): Promise<void> {
    const user = await this.userRepository.findOne({
      where: { id: userId },
    });

    if (!user) {
      throw new NotFoundException('Usuario no encontrado');
    }

    // Asegurar límites
    const finalScore = Math.max(
      this.ALGORITHM.MIN_SCORE,
      Math.min(this.ALGORITHM.MAX_SCORE, newScore),
    );

    // ⚠️ IMPORTANTE: Esto solo actualiza un campo en la base de datos.
    // NO afecta fondos, NO ejecuta transacciones blockchain.
    user.reputationScore = finalScore;
    await this.userRepository.save(user);

    this.logger.log(
      `Reputation score updated for user ${userId}: ${finalScore}`,
    );
  }

  /**
   * Obtiene el ranking de usuarios por reputation
   * 
   * ⚠️ REGLA FINAL: Este método SOLO ordena usuarios por score.
   * NUNCA afecta fondos o ejecuta transacciones blockchain.
   */
  async getRanking(limit: number = 100): Promise<{
    userId: string;
    walletAddress: string;
    score: number;
    rank: number;
  }[]> {
    const users = await this.userRepository.find({
      where: { isActive: true },
      order: { reputationScore: 'DESC' },
      take: limit,
    });

    return users.map((user, index) => ({
      userId: user.id,
      walletAddress: user.walletAddress,
      score: Number(user.reputationScore),
      rank: index + 1,
    }));
  }

  /**
   * Obtiene información detallada sobre el algoritmo de scoring
   * 
   * Útil para transparencia y explicar cómo se calcula el score
   */
  getAlgorithmInfo(): {
    scoring: Record<string, number>;
    algorithm: {
      minScore: number;
      maxScore: number;
      decayDays: number;
      volumeBonusThreshold: number;
      volumeBonusMultiplier: number;
      successRateBonusThreshold: number;
      successRateBonus: number;
      repeatOffenderPenalty: number;
    };
    description: string;
  } {
    return {
      scoring: { ...this.SCORING },
      algorithm: {
        minScore: this.ALGORITHM.MIN_SCORE,
        maxScore: this.ALGORITHM.MAX_SCORE,
        decayDays: this.ALGORITHM.DECAY_DAYS,
        volumeBonusThreshold: this.ALGORITHM.VOLUME_BONUS_THRESHOLD,
        volumeBonusMultiplier: this.ALGORITHM.VOLUME_BONUS_MULTIPLIER,
        successRateBonusThreshold: this.ALGORITHM.SUCCESS_RATE_BONUS_THRESHOLD,
        successRateBonus: this.ALGORITHM.SUCCESS_RATE_BONUS,
        repeatOffenderPenalty: this.ALGORITHM.REPEAT_OFFENDER_PENALTY,
      },
      description: `
        El algoritmo de reputation considera:
        1. Eventos base: Cada evento tiene un valor base (trade completado, cancelado, etc.)
        2. Decaimiento temporal: Eventos más recientes pesan más que eventos antiguos
        3. Bonus por volumen: Usuarios con muchos trades reciben un bonus
        4. Bonus por tasa de éxito: Usuarios con alta tasa de éxito reciben bonus adicional
        5. Penalización por comportamiento repetitivo: Múltiples eventos negativos recientes aumentan la penalización
        
        ⚠️ IMPORTANTE: Este score es puramente informativo.
        NO afecta fondos, NO ejecuta transacciones blockchain.
        Solo ayuda a los usuarios a tomar decisiones informadas.
      `.trim(),
    };
  }

  /**
   * Obtiene un análisis detallado del score de un usuario
   * 
   * Muestra cómo se calculó el score y qué factores influyeron
   */
  async getScoreAnalysis(userId: string): Promise<{
    userId: string;
    currentScore: number;
    baseScore: number;
    adjustments: {
      temporalDecay: number;
      volumeBonus: number;
      successRateBonus: number;
      repeatOffenderPenalty: number;
    };
    statistics: {
      totalEvents: number;
      completedTrades: number;
      cancelledTrades: number;
      disputesOpened: number;
      successRate: number;
      recentNegativeEvents: number;
    };
  }> {
    const user = await this.userRepository.findOne({
      where: { id: userId },
    });

    if (!user) {
      throw new NotFoundException('Usuario no encontrado');
    }

    const events = await this.eventRepository.find({
      where: { userId },
      order: { createdAt: 'ASC' },
    });

    // Calcular score base (sin ajustes)
    let baseScore = 0;
    events.forEach((event) => {
      baseScore += Number(event.scoreChange);
    });

    // Calcular estadísticas
    const now = new Date();
    let completedTrades = 0;
    let cancelledTrades = 0;
    let disputesOpened = 0;
    let recentNegativeEvents = 0;

    events.forEach((event) => {
      if (event.eventType === ReputationEventType.TRADE_COMPLETED) {
        completedTrades++;
      } else if (event.eventType === ReputationEventType.TRADE_CANCELLED) {
        cancelledTrades++;
      } else if (event.eventType === ReputationEventType.DISPUTE_OPENED) {
        disputesOpened++;
      }

      const daysAgo = Math.floor(
        (now.getTime() - event.createdAt.getTime()) / (1000 * 60 * 60 * 24),
      );

      if (Number(event.scoreChange) < 0 && daysAgo <= 30) {
        recentNegativeEvents++;
      }
    });

    const totalTrades = completedTrades + cancelledTrades;
    const successRate = totalTrades > 0 ? completedTrades / totalTrades : 0;

    // Calcular ajustes
    const temporalDecay = 0; // Se calcula en el algoritmo avanzado
    const volumeBonus =
      totalTrades >= this.ALGORITHM.VOLUME_BONUS_THRESHOLD
        ? baseScore * (this.ALGORITHM.VOLUME_BONUS_MULTIPLIER - 1)
        : 0;
    const successRateBonus =
      successRate >= this.ALGORITHM.SUCCESS_RATE_BONUS_THRESHOLD
        ? this.ALGORITHM.SUCCESS_RATE_BONUS
        : 0;
    const repeatOffenderPenalty =
      recentNegativeEvents >= 3
        ? baseScore * (this.ALGORITHM.REPEAT_OFFENDER_PENALTY - 1)
        : 0;

    return {
      userId: user.id,
      currentScore: Number(user.reputationScore),
      baseScore: Math.round(baseScore * 100) / 100,
      adjustments: {
        temporalDecay,
        volumeBonus: Math.round(volumeBonus * 100) / 100,
        successRateBonus,
        repeatOffenderPenalty: Math.round(repeatOffenderPenalty * 100) / 100,
      },
      statistics: {
        totalEvents: events.length,
        completedTrades,
        cancelledTrades,
        disputesOpened,
        successRate: Math.round(successRate * 10000) / 100,
        recentNegativeEvents,
      },
    };
  }
}