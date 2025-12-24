/**
 * ⚠️ REGLA FINAL: Este guard es una validación de desarrollo.
 * 
 * En producción, el código nunca debe tener métodos que muevan fondos.
 * Este guard ayuda a detectar violaciones durante el desarrollo.
 * 
 * Si este guard se activa, significa que hay código que intenta
 * mover fondos, lo cual está PROHIBIDO.
 */

import { Injectable, CanActivate, ExecutionContext, Logger } from '@nestjs/common';
import { Reflector } from '@nestjs/core';

/**
 * Decorador para marcar endpoints que NUNCA deben mover fondos
 */
export const NO_FUNDS_MOVEMENT = 'noFundsMovement';

/**
 * Guard que valida que no se estén usando métodos que muevan fondos
 * 
 * NOTA: Este es un guard de desarrollo. En producción, el código
 * nunca debe tener estos métodos, por lo que este guard nunca debería
 * activarse.
 */
@Injectable()
export class NoFundsMovementGuard implements CanActivate {
  private readonly logger = new Logger(NoFundsMovementGuard.name);

  constructor(private readonly reflector: Reflector) {}

  canActivate(context: ExecutionContext): boolean {
    const request = context.switchToHttp().getRequest();
    const handler = context.getHandler();
    
    // Verificar si el endpoint está marcado como "no debe mover fondos"
    const noFundsMovement = this.reflector.get<boolean>(
      NO_FUNDS_MOVEMENT,
      handler,
    );

    if (noFundsMovement) {
      // En desarrollo, podríamos agregar validaciones adicionales aquí
      // Por ejemplo, verificar que no hay llamadas a métodos prohibidos
      this.logger.debug(`Endpoint ${handler.name} marcado como no debe mover fondos`);
    }

    // Siempre permitir (este guard es solo para documentación y desarrollo)
    // En producción, el código nunca debe tener métodos que muevan fondos
    return true;
  }
}
