/**
 * Estados canónicos de una orden
 * 
 * Rol: Estados canónicos
 * 
 * Contiene:
 * - CREATED: Orden creada, esperando aceptación
 * - AWAITING_FUNDS: Orden aceptada, esperando fondos en escrow
 * - ONCHAIN_LOCKED: Fondos bloqueados en escrow on-chain
 * - COMPLETED: Orden completada exitosamente
 * - REFUNDED: Orden cancelada/reembolsada
 * - DISPUTED: Orden en disputa
 * 
 * ⚠️ REGLA FINAL: Estos estados son puramente informativos.
 * NO afectan fondos ni ejecutan transacciones blockchain.
 * Solo representan el estado off-chain de una orden.
 */
export enum OrderStatus {
  /**
   * Orden creada
   * 
   * Estado inicial cuando un vendedor crea una oferta.
   * La orden está disponible para ser aceptada por un comprador.
   */
  CREATED = 'CREATED',

  /**
   * Esperando fondos
   * 
   * Orden aceptada por un comprador.
   * El comprador debe depositar fondos en el escrow.
   * Estado transitorio antes de ONCHAIN_LOCKED.
   */
  AWAITING_FUNDS = 'AWAITING_FUNDS',

  /**
   * Fondos bloqueados on-chain
   * 
   * Los fondos han sido bloqueados en el contrato escrow.
   * Estado confirmado por eventos de blockchain.
   * La orden está lista para completarse.
   */
  ONCHAIN_LOCKED = 'ONCHAIN_LOCKED',

  /**
   * Orden completada
   * 
   * La orden fue completada exitosamente.
   * Los fondos fueron liberados del escrow al vendedor.
   * Estado final exitoso.
   */
  COMPLETED = 'COMPLETED',

  /**
   * Orden reembolsada
   * 
   * La orden fue cancelada o reembolsada.
   * Los fondos fueron devueltos al comprador.
   * Estado final de cancelación.
   */
  REFUNDED = 'REFUNDED',

  /**
   * Orden en disputa
   * 
   * Se ha abierto una disputa para esta orden.
   * Requiere resolución manual o por el contrato escrow.
   * Puede transicionar a COMPLETED o REFUNDED después de la resolución.
   */
  DISPUTED = 'DISPUTED',
}

/**
 * Transiciones válidas de estados
 * 
 * Define qué transiciones de estado son permitidas
 */
export const VALID_TRANSITIONS: Record<OrderStatus, OrderStatus[]> = {
  [OrderStatus.CREATED]: [
    OrderStatus.AWAITING_FUNDS,
    OrderStatus.REFUNDED, // Puede cancelarse antes de aceptar
  ],
  [OrderStatus.AWAITING_FUNDS]: [
    OrderStatus.ONCHAIN_LOCKED,
    OrderStatus.REFUNDED, // Puede cancelarse antes de bloquear fondos
  ],
  [OrderStatus.ONCHAIN_LOCKED]: [
    OrderStatus.COMPLETED,
    OrderStatus.REFUNDED,
    OrderStatus.DISPUTED,
  ],
  [OrderStatus.COMPLETED]: [], // Estado final, no puede cambiar
  [OrderStatus.REFUNDED]: [], // Estado final, no puede cambiar
  [OrderStatus.DISPUTED]: [
    OrderStatus.COMPLETED, // Disputa resuelta a favor del vendedor
    OrderStatus.REFUNDED, // Disputa resuelta a favor del comprador
  ],
};

/**
 * Estados finales (no pueden cambiar)
 */
export const FINAL_STATUSES: OrderStatus[] = [
  OrderStatus.COMPLETED,
  OrderStatus.REFUNDED,
];

/**
 * Estados activos (orden en progreso)
 */
export const ACTIVE_STATUSES: OrderStatus[] = [
  OrderStatus.CREATED,
  OrderStatus.AWAITING_FUNDS,
  OrderStatus.ONCHAIN_LOCKED,
  OrderStatus.DISPUTED,
];

/**
 * Utilidades para trabajar con estados de orden
 */
export class OrderStatusUtil {
  /**
   * Verifica si una transición de estado es válida
   */
  static isValidTransition(
    from: OrderStatus,
    to: OrderStatus,
  ): boolean {
    if (from === to) {
      return true; // Permite mantener el mismo estado
    }

    const validTransitions = VALID_TRANSITIONS[from];
    return validTransitions.includes(to);
  }

  /**
   * Verifica si un estado es final (no puede cambiar)
   */
  static isFinalStatus(status: OrderStatus): boolean {
    return FINAL_STATUSES.includes(status);
  }

  /**
   * Verifica si un estado es activo (orden en progreso)
   */
  static isActiveStatus(status: OrderStatus): boolean {
    return ACTIVE_STATUSES.includes(status);
  }

  /**
   * Obtiene los estados a los que se puede transicionar desde un estado dado
   */
  static getValidNextStates(from: OrderStatus): OrderStatus[] {
    return VALID_TRANSITIONS[from] || [];
  }

  /**
   * Obtiene una descripción legible del estado
   */
  static getStatusDescription(status: OrderStatus): string {
    const descriptions: Record<OrderStatus, string> = {
      [OrderStatus.CREATED]: 'Orden creada, esperando aceptación',
      [OrderStatus.AWAITING_FUNDS]: 'Orden aceptada, esperando fondos en escrow',
      [OrderStatus.ONCHAIN_LOCKED]: 'Fondos bloqueados en escrow on-chain',
      [OrderStatus.COMPLETED]: 'Orden completada exitosamente',
      [OrderStatus.REFUNDED]: 'Orden cancelada/reembolsada',
      [OrderStatus.DISPUTED]: 'Orden en disputa',
    };

    return descriptions[status] || 'Estado desconocido';
  }

  /**
   * Obtiene el color/etiqueta para UI (opcional)
   */
  static getStatusLabel(status: OrderStatus): {
    label: string;
    color: string;
    variant: 'success' | 'warning' | 'error' | 'info' | 'default';
  } {
    const labels: Record<OrderStatus, { label: string; color: string; variant: 'success' | 'warning' | 'error' | 'info' | 'default' }> = {
      [OrderStatus.CREATED]: {
        label: 'Creada',
        color: '#6B7280',
        variant: 'default',
      },
      [OrderStatus.AWAITING_FUNDS]: {
        label: 'Esperando Fondos',
        color: '#F59E0B',
        variant: 'warning',
      },
      [OrderStatus.ONCHAIN_LOCKED]: {
        label: 'Bloqueada',
        color: '#3B82F6',
        variant: 'info',
      },
      [OrderStatus.COMPLETED]: {
        label: 'Completada',
        color: '#10B981',
        variant: 'success',
      },
      [OrderStatus.REFUNDED]: {
        label: 'Reembolsada',
        color: '#EF4444',
        variant: 'error',
      },
      [OrderStatus.DISPUTED]: {
        label: 'En Disputa',
        color: '#F59E0B',
        variant: 'warning',
      },
    };

    return labels[status];
  }

  /**
   * Valida que un string sea un estado válido
   */
  static isValidStatus(status: string): status is OrderStatus {
    return Object.values(OrderStatus).includes(status as OrderStatus);
  }

  /**
   * Obtiene todos los estados posibles
   */
  static getAllStatuses(): OrderStatus[] {
    return Object.values(OrderStatus);
  }
}

