import { Injectable, Logger, Inject } from '@nestjs/common';
import { Redis } from 'ioredis';
import { ConfigService } from '@nestjs/config';

/**
 * Audit Service
 * 
 * Sistema de auditoría de seguridad para registrar acciones críticas:
 * - Creación de órdenes
 * - Aceptación de órdenes
 * - Cambios de estado críticos
 * - Intentos de acceso fallidos
 * - Acciones administrativas
 */
@Injectable()
export class AuditService {
  private readonly logger = new Logger(AuditService.name);
  private readonly auditEnabled: boolean;
  private readonly auditTtl: number = 2592000; // 30 días

  constructor(
    @Inject('REDIS_CLIENT')
    private readonly redis: Redis,
    private readonly configService: ConfigService,
  ) {
    this.auditEnabled =
      this.configService.get<string>('AUDIT_ENABLED') !== 'false';
  }

  /**
   * Registra un evento de auditoría
   */
  async log(
    action: AuditAction,
    userId: string,
    details: AuditDetails,
  ): Promise<void> {
    if (!this.auditEnabled) {
      return;
    }

    try {
      const auditEntry: AuditEntry = {
        id: this.generateId(),
        action,
        userId,
        timestamp: new Date().toISOString(),
        ip: details.ip,
        userAgent: details.userAgent,
        resourceType: details.resourceType,
        resourceId: details.resourceId,
        metadata: details.metadata || {},
        success: details.success !== false,
        error: details.error,
      };

      // Guardar en Redis con TTL
      const key = `audit:${auditEntry.id}`;
      await this.redis.setex(
        key,
        this.auditTtl,
        JSON.stringify(auditEntry),
      );

      // También guardar índice por usuario
      const userKey = `audit:user:${userId}`;
      await this.redis.lpush(userKey, auditEntry.id);
      await this.redis.expire(userKey, this.auditTtl);
      await this.redis.ltrim(userKey, 0, 999); // Mantener solo últimos 1000

      // También guardar índice por acción
      const actionKey = `audit:action:${action}`;
      await this.redis.lpush(actionKey, auditEntry.id);
      await this.redis.expire(actionKey, this.auditTtl);
      await this.redis.ltrim(actionKey, 0, 999);

      // Log estructurado
      this.logger.log(
        `Audit: ${action} by ${userId} on ${details.resourceType}:${details.resourceId}`,
        {
          auditId: auditEntry.id,
          action,
          userId,
          resourceType: details.resourceType,
          resourceId: details.resourceId,
          success: auditEntry.success,
        },
      );
    } catch (error) {
      this.logger.error(`Failed to log audit entry: ${error.message}`, error.stack);
    }
  }

  /**
   * Registra creación de orden
   */
  async logOrderCreated(
    userId: string,
    orderId: string,
    details: Partial<AuditDetails>,
  ): Promise<void> {
    await this.log(
      AuditAction.ORDER_CREATED,
      userId,
      {
        resourceType: 'order',
        resourceId: orderId,
        ...details,
      },
    );
  }

  /**
   * Registra aceptación de orden
   */
  async logOrderAccepted(
    userId: string,
    orderId: string,
    details: Partial<AuditDetails>,
  ): Promise<void> {
    await this.log(
      AuditAction.ORDER_ACCEPTED,
      userId,
      {
        resourceType: 'order',
        resourceId: orderId,
        ...details,
      },
    );
  }

  /**
   * Registra cancelación de orden
   */
  async logOrderCancelled(
    userId: string,
    orderId: string,
    details: Partial<AuditDetails>,
  ): Promise<void> {
    await this.log(
      AuditAction.ORDER_CANCELLED,
      userId,
      {
        resourceType: 'order',
        resourceId: orderId,
        ...details,
      },
    );
  }

  /**
   * Registra intento de acceso fallido
   */
  async logAccessDenied(
    userId: string | null,
    resourceType: string,
    resourceId: string,
    details: Partial<AuditDetails>,
  ): Promise<void> {
    await this.log(
      AuditAction.ACCESS_DENIED,
      userId || 'anonymous',
      {
        resourceType,
        resourceId,
        success: false,
        ...details,
      },
    );
  }

  /**
   * Registra cambio de estado crítico
   */
  async logStatusChange(
    userId: string,
    resourceType: string,
    resourceId: string,
    oldStatus: string,
    newStatus: string,
    details: Partial<AuditDetails>,
  ): Promise<void> {
    await this.log(
      AuditAction.STATUS_CHANGED,
      userId,
      {
        resourceType,
        resourceId,
        metadata: {
          oldStatus,
          newStatus,
          ...details.metadata,
        },
        ...details,
      },
    );
  }

  /**
   * Obtiene eventos de auditoría de un usuario
   */
  async getUserAuditLogs(
    userId: string,
    limit: number = 100,
  ): Promise<AuditEntry[]> {
    const userKey = `audit:user:${userId}`;
    const auditIds = await this.redis.lrange(userKey, 0, limit - 1);

    const entries: AuditEntry[] = [];
    for (const id of auditIds) {
      const key = `audit:${id}`;
      const data = await this.redis.get(key);
      if (data) {
        entries.push(JSON.parse(data));
      }
    }

    return entries;
  }

  /**
   * Obtiene eventos de auditoría por acción
   */
  async getActionAuditLogs(
    action: AuditAction,
    limit: number = 100,
  ): Promise<AuditEntry[]> {
    const actionKey = `audit:action:${action}`;
    const auditIds = await this.redis.lrange(actionKey, 0, limit - 1);

    const entries: AuditEntry[] = [];
    for (const id of auditIds) {
      const key = `audit:${id}`;
      const data = await this.redis.get(key);
      if (data) {
        entries.push(JSON.parse(data));
      }
    }

    return entries;
  }

  /**
   * Genera un ID único para el evento de auditoría
   */
  private generateId(): string {
    return `audit_${Date.now()}_${Math.random().toString(36).substring(2, 15)}`;
  }
}

/**
 * Acciones auditables
 */
export enum AuditAction {
  ORDER_CREATED = 'ORDER_CREATED',
  ORDER_ACCEPTED = 'ORDER_ACCEPTED',
  ORDER_CANCELLED = 'ORDER_CANCELLED',
  ORDER_COMPLETED = 'ORDER_COMPLETED',
  ORDER_DISPUTED = 'ORDER_DISPUTED',
  STATUS_CHANGED = 'STATUS_CHANGED',
  ACCESS_DENIED = 'ACCESS_DENIED',
  LOGIN_SUCCESS = 'LOGIN_SUCCESS',
  LOGIN_FAILED = 'LOGIN_FAILED',
  DISPUTE_CREATED = 'DISPUTE_CREATED',
  DISPUTE_RESOLVED = 'DISPUTE_RESOLVED',
}

/**
 * Detalles del evento de auditoría
 */
export interface AuditDetails {
  ip?: string;
  userAgent?: string;
  resourceType: string;
  resourceId: string;
  metadata?: Record<string, any>;
  success?: boolean;
  error?: string;
}

/**
 * Entrada de auditoría
 */
export interface AuditEntry {
  id: string;
  action: AuditAction;
  userId: string;
  timestamp: string;
  ip?: string;
  userAgent?: string;
  resourceType: string;
  resourceId: string;
  metadata: Record<string, any>;
  success: boolean;
  error?: string;
}

