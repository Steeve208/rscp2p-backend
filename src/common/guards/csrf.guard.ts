import {
  Injectable,
  CanActivate,
  ExecutionContext,
  BadRequestException,
} from '@nestjs/common';
import { Request } from 'express';
import { ConfigService } from '@nestjs/config';

/**
 * CSRF Guard
 * 
 * Protege contra ataques CSRF validando tokens CSRF en requests que modifican datos
 */
@Injectable()
export class CsrfGuard implements CanActivate {
  private readonly csrfEnabled: boolean;
  private readonly csrfHeaderName: string = 'X-CSRF-Token';
  private readonly csrfCookieName: string = 'csrf-token';

  constructor(private readonly configService: ConfigService) {
    this.csrfEnabled =
      this.configService.get<string>('CSRF_ENABLED') !== 'false';
  }

  canActivate(context: ExecutionContext): boolean {
    if (!this.csrfEnabled) {
      return true; // CSRF deshabilitado
    }

    const request = context.switchToHttp().getRequest<Request>();
    const method = request.method;

    // Solo validar métodos que modifican datos
    if (!['POST', 'PUT', 'PATCH', 'DELETE'].includes(method)) {
      return true;
    }

    // Obtener token del header
    const tokenFromHeader = request.headers[this.csrfHeaderName.toLowerCase()] as string;
    
    // Obtener token de la cookie
    const tokenFromCookie = request.cookies?.[this.csrfCookieName];

    // Validar que ambos tokens existen y coinciden
    if (!tokenFromHeader || !tokenFromCookie) {
      throw new BadRequestException('CSRF token missing');
    }

    if (tokenFromHeader !== tokenFromCookie) {
      throw new BadRequestException('CSRF token mismatch');
    }

    return true;
  }
}

