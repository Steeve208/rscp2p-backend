import {
  Injectable,
  NestInterceptor,
  ExecutionContext,
  CallHandler,
} from '@nestjs/common';
import { Observable } from 'rxjs';
import { tap } from 'rxjs/operators';
import { AuditService, AuditAction } from '../audit/audit.service';
import { Request } from 'express';

/**
 * Audit Interceptor
 * 
 * Intercepta requests y registra eventos de auditoría automáticamente
 */
@Injectable()
export class AuditInterceptor implements NestInterceptor {
  constructor(private readonly auditService: AuditService) {}

  intercept(context: ExecutionContext, next: CallHandler): Observable<any> {
    const request = context.switchToHttp().getRequest<Request>();
    const { method, url, ip, headers } = request;
    const user = (request as any).user;

    // Solo auditar métodos que modifican datos
    if (!['POST', 'PUT', 'PATCH', 'DELETE'].includes(method)) {
      return next.handle();
    }

    const userId = (user as any)?.id || (user as any)?.sub || null;
    const userAgent = headers['user-agent'] || '';

    return next.handle().pipe(
      tap({
        next: async (data) => {
          // Determinar acción de auditoría basada en la URL
          const action = this.getActionFromUrl(method, url);
          if (action && userId) {
            const resourceId = this.extractResourceId(url, data);
            const resourceType = this.extractResourceType(url);

            await this.auditService.log(action, userId, {
              ip,
              userAgent,
              resourceType,
              resourceId,
              metadata: {
                method,
                url,
              },
              success: true,
            });
          }
        },
        error: async (error) => {
          // Registrar errores de acceso denegado
          if (error.status === 403 || error.status === 401) {
            const resourceId = this.extractResourceId(url, null);
            const resourceType = this.extractResourceType(url);

            await this.auditService.logAccessDenied(
              userId,
              resourceType,
              resourceId,
              {
                ip,
                userAgent,
                error: error.message,
              },
            );
          }
        },
      }),
    );
  }

  /**
   * Determina la acción de auditoría basada en la URL y método
   */
  private getActionFromUrl(method: string, url: string): AuditAction | null {
    if (url.includes('/orders')) {
      if (method === 'POST') return AuditAction.ORDER_CREATED;
      if (url.includes('/accept') && method === 'PUT') return AuditAction.ORDER_ACCEPTED;
      if (url.includes('/cancel') && method === 'PUT') return AuditAction.ORDER_CANCELLED;
      if (url.includes('/complete') && method === 'PUT') return AuditAction.ORDER_COMPLETED;
      if (url.includes('/dispute') && method === 'PUT') return AuditAction.ORDER_DISPUTED;
    }
    if (url.includes('/disputes')) {
      if (method === 'POST') return AuditAction.DISPUTE_CREATED;
      if (url.includes('/resolve') && method === 'PUT') return AuditAction.DISPUTE_RESOLVED;
    }
    return null;
  }

  /**
   * Extrae el ID del recurso de la URL o respuesta
   */
  private extractResourceId(url: string, data: any): string {
    // Intentar extraer de la URL
    const urlMatch = url.match(/\/([a-f0-9-]{36})/);
    if (urlMatch) {
      return urlMatch[1];
    }

    // Intentar extraer de la respuesta
    if (data?.id) {
      return data.id;
    }
    if (data?.data?.id) {
      return data.data.id;
    }

    return 'unknown';
  }

  /**
   * Extrae el tipo de recurso de la URL
   */
  private extractResourceType(url: string): string {
    if (url.includes('/orders')) return 'order';
    if (url.includes('/disputes')) return 'dispute';
    if (url.includes('/users')) return 'user';
    if (url.includes('/escrow')) return 'escrow';
    return 'unknown';
  }
}

