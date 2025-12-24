import { Injectable, Inject, Logger, OnModuleInit, OnModuleDestroy } from '@nestjs/common';
import { ConfigService } from '@nestjs/config';
import { ethers } from 'ethers';
import { ESCROW_EVENTS_ABI } from '../../config/blockchain';
import { EscrowService } from '../escrow/escrow.service';
import { EventListenerService } from './listeners/event-listener.service';
import { StateReconcilerService } from './reconcilers/state-reconciler.service';
import { EscrowStatus } from '../../common/enums/escrow-status.enum';

/**
 * Oyente de eventos on-chain
 * 
 * Rol: Oyente de eventos on-chain
 * 
 * Contiene:
 * - Subscripción a eventos del contrato
 * - Reconciliación de estados
 * 
 * Se conecta con:
 * - RPC (blockchainProvider.provider)
 * - escrow.service (EscrowService)
 * 
 * ⚠️ REGLA FINAL: Este servicio SOLO escucha eventos de blockchain.
 * NUNCA ejecuta transacciones que muevan fondos.
 * 
 * Las transacciones son ejecutadas por los usuarios desde el frontend.
 * Este servicio solo reacciona a los eventos emitidos.
 */
@Injectable()
export class BlockchainListener implements OnModuleInit, OnModuleDestroy {
  private readonly logger = new Logger(BlockchainListener.name);
  private contract: ethers.Contract | null = null;
  private listeners: Map<string, ethers.ContractEventPayload[]> = new Map();
  private isListening = false;

  constructor(
    @Inject('BLOCKCHAIN_PROVIDER')
    private readonly blockchainProvider: {
      provider: ethers.Provider;
      escrowContractAddress: string;
    },
    private readonly configService: ConfigService,
    private readonly escrowService: EscrowService,
    private readonly eventListener: EventListenerService,
    private readonly stateReconciler: StateReconcilerService,
  ) {}

  /**
   * Inicializa el listener cuando el módulo se inicia
   */
  async onModuleInit() {
    this.logger.log('Initializing blockchain listener...');
    await this.startListening();
  }

  /**
   * Detiene el listener cuando el módulo se destruye
   */
  async onModuleDestroy() {
    this.logger.log('Stopping blockchain listener...');
    await this.stopListening();
  }

  /**
   * Inicia la subscripción a eventos del contrato
   * 
   * Se conecta con: RPC (blockchainProvider.provider)
   */
  async startListening(): Promise<void> {
    if (this.isListening) {
      this.logger.warn('Listener is already running');
      return;
    }

    const contractAddress = this.blockchainProvider.escrowContractAddress;

    if (!contractAddress) {
      this.logger.warn('No escrow contract address configured, skipping event listener');
      return;
    }

    try {
      // Crear instancia del contrato conectada al RPC
      this.contract = new ethers.Contract(
        contractAddress,
        ESCROW_EVENTS_ABI,
        this.blockchainProvider.provider,
      );

      // Subscribirse a eventos del contrato
      await this.subscribeToEvents();

      this.isListening = true;
      this.logger.log(`Blockchain listener started for contract ${contractAddress}`);
    } catch (error) {
      this.logger.error(`Error starting blockchain listener: ${error.message}`, error.stack);
      throw error;
    }
  }

  /**
   * Subscribirse a todos los eventos del contrato escrow
   * 
   * Se conecta con: RPC (contract.on())
   */
  private async subscribeToEvents(): Promise<void> {
    if (!this.contract) {
      throw new Error('Contract not initialized');
    }

    // Listener para EscrowCreated
    this.contract.on('EscrowCreated', async (escrowId, seller, buyer, amount, event) => {
      await this.handleEscrowCreated(
        escrowId.toString(),
        seller.toString(),
        buyer.toString(),
        amount.toString(),
        event,
      );
    });

    // Listener para FundsLocked
    this.contract.on('FundsLocked', async (escrowId, amount, event) => {
      await this.handleFundsLocked(escrowId.toString(), amount.toString(), event);
    });

    // Listener para FundsReleased
    this.contract.on('FundsReleased', async (escrowId, recipient, amount, event) => {
      await this.handleFundsReleased(
        escrowId.toString(),
        recipient.toString(),
        amount.toString(),
        event,
      );
    });

    // Listener para FundsRefunded
    this.contract.on('FundsRefunded', async (escrowId, recipient, amount, event) => {
      await this.handleFundsRefunded(
        escrowId.toString(),
        recipient.toString(),
        amount.toString(),
        event,
      );
    });

    // Listener para DisputeOpened
    this.contract.on('DisputeOpened', async (escrowId, initiator, event) => {
      await this.handleDisputeOpened(escrowId.toString(), initiator.toString(), event);
    });

    // Listener para DisputeResolved
    this.contract.on('DisputeResolved', async (escrowId, resolver, resolution, event) => {
      await this.handleDisputeResolved(
        escrowId.toString(),
        resolver.toString(),
        resolution.toString(),
        event,
      );
    });

    this.logger.log('Subscribed to all escrow contract events');
  }

  /**
   * Maneja evento EscrowCreated
   * 
   * Se conecta con: escrow.service (EscrowService.handleEscrowCreated)
   */
  private async handleEscrowCreated(
    escrowId: string,
    seller: string,
    buyer: string,
    amount: string,
    event: ethers.Log,
  ): Promise<void> {
    try {
      this.logger.log(`EscrowCreated event: ${escrowId} - Seller: ${seller}, Buyer: ${buyer}, Amount: ${amount}`);

      // Guardar evento en base de datos (a través de EventListenerService)
      await this.eventListener.saveEvent('EscrowCreated', event, {
        escrowId,
        seller,
        buyer,
        amount,
      });

      // Intentar crear el mapeo en escrow.service si no existe
      // Nota: Esto requiere que el orderId esté disponible, lo cual puede no ser el caso
      // Por lo tanto, solo guardamos el evento y la reconciliación lo procesará después
      
      // Reconciliar el estado
      await this.reconcileEscrowState(escrowId);
    } catch (error) {
      this.logger.error(
        `Error handling EscrowCreated event: ${error.message}`,
        error.stack,
      );
    }
  }

  /**
   * Maneja evento FundsLocked
   * 
   * Se conecta con: escrow.service (EscrowService.handleFundsLocked)
   */
  private async handleFundsLocked(
    escrowId: string,
    amount: string,
    event: ethers.Log,
  ): Promise<void> {
    try {
      this.logger.log(`FundsLocked event: ${escrowId} - Amount: ${amount}`);

      // Guardar evento en base de datos
      await this.eventListener.saveEvent('FundsLocked', event, {
        escrowId,
        amount,
      });

      // Actualizar estado en escrow.service
      try {
        await this.escrowService.handleFundsLocked(escrowId, event.transactionHash);
        this.logger.log(`Escrow ${escrowId} marked as LOCKED`);
      } catch (error) {
        // Si el escrow no existe aún, la reconciliación lo procesará
        this.logger.warn(`Could not update escrow ${escrowId}: ${error.message}`);
      }

      // Reconciliar el estado
      await this.reconcileEscrowState(escrowId);
    } catch (error) {
      this.logger.error(
        `Error handling FundsLocked event: ${error.message}`,
        error.stack,
      );
    }
  }

  /**
   * Maneja evento FundsReleased
   * 
   * Se conecta con: escrow.service (EscrowService.handleFundsReleased)
   */
  private async handleFundsReleased(
    escrowId: string,
    recipient: string,
    amount: string,
    event: ethers.Log,
  ): Promise<void> {
    try {
      this.logger.log(`FundsReleased event: ${escrowId} - Recipient: ${recipient}, Amount: ${amount}`);

      // Guardar evento en base de datos
      await this.eventListener.saveEvent('FundsReleased', event, {
        escrowId,
        recipient,
        amount,
      });

      // Actualizar estado en escrow.service
      try {
        await this.escrowService.handleFundsReleased(escrowId, event.transactionHash);
        this.logger.log(`Escrow ${escrowId} marked as RELEASED`);
      } catch (error) {
        this.logger.warn(`Could not update escrow ${escrowId}: ${error.message}`);
      }

      // Reconciliar el estado
      await this.reconcileEscrowState(escrowId);
    } catch (error) {
      this.logger.error(
        `Error handling FundsReleased event: ${error.message}`,
        error.stack,
      );
    }
  }

  /**
   * Maneja evento FundsRefunded
   * 
   * Se conecta con: escrow.service (EscrowService.handleFundsRefunded)
   */
  private async handleFundsRefunded(
    escrowId: string,
    recipient: string,
    amount: string,
    event: ethers.Log,
  ): Promise<void> {
    try {
      this.logger.log(`FundsRefunded event: ${escrowId} - Recipient: ${recipient}, Amount: ${amount}`);

      // Guardar evento en base de datos
      await this.eventListener.saveEvent('FundsRefunded', event, {
        escrowId,
        recipient,
        amount,
      });

      // Actualizar estado en escrow.service
      try {
        await this.escrowService.handleFundsRefunded(escrowId, event.transactionHash);
        this.logger.log(`Escrow ${escrowId} marked as REFUNDED`);
      } catch (error) {
        this.logger.warn(`Could not update escrow ${escrowId}: ${error.message}`);
      }

      // Reconciliar el estado
      await this.reconcileEscrowState(escrowId);
    } catch (error) {
      this.logger.error(
        `Error handling FundsRefunded event: ${error.message}`,
        error.stack,
      );
    }
  }

  /**
   * Maneja evento DisputeOpened
   * 
   * Se conecta con: escrow.service (para actualizar estado)
   */
  private async handleDisputeOpened(
    escrowId: string,
    initiator: string,
    event: ethers.Log,
  ): Promise<void> {
    try {
      this.logger.log(`DisputeOpened event: ${escrowId} - Initiator: ${initiator}`);

      // Guardar evento en base de datos
      await this.eventListener.saveEvent('DisputeOpened', event, {
        escrowId,
        initiator,
      });

      // Actualizar estado en escrow.service
      try {
        await this.escrowService.update(escrowId, {
          status: EscrowStatus.DISPUTED,
        });
        this.logger.log(`Escrow ${escrowId} marked as DISPUTED`);
      } catch (error) {
        this.logger.warn(`Could not update escrow ${escrowId}: ${error.message}`);
      }

      // Reconciliar el estado
      await this.reconcileEscrowState(escrowId);
    } catch (error) {
      this.logger.error(
        `Error handling DisputeOpened event: ${error.message}`,
        error.stack,
      );
    }
  }

  /**
   * Maneja evento DisputeResolved
   */
  private async handleDisputeResolved(
    escrowId: string,
    resolver: string,
    resolution: string,
    event: ethers.Log,
  ): Promise<void> {
    try {
      this.logger.log(`DisputeResolved event: ${escrowId} - Resolver: ${resolver}, Resolution: ${resolution}`);

      // Guardar evento en base de datos
      await this.eventListener.saveEvent('DisputeResolved', event, {
        escrowId,
        resolver,
        resolution,
      });

      // Reconciliar el estado (el estado final dependerá de la resolución)
      await this.reconcileEscrowState(escrowId);
    } catch (error) {
      this.logger.error(
        `Error handling DisputeResolved event: ${error.message}`,
        error.stack,
      );
    }
  }

  /**
   * Reconcilia el estado de un escrow
   * 
   * Se conecta con: stateReconciler (StateReconcilerService.reconcileEscrow)
   */
  private async reconcileEscrowState(escrowId: string): Promise<void> {
    try {
      const result = await this.stateReconciler.reconcileEscrow(escrowId);
      
      if (result.reconciled && result.changes.length > 0) {
        this.logger.log(
          `Reconciled escrow ${escrowId}: ${result.changes.join(', ')}`,
        );
      }
    } catch (error) {
      this.logger.error(
        `Error reconciling escrow ${escrowId}: ${error.message}`,
        error.stack,
      );
    }
  }

  /**
   * Reconcilia todos los estados pendientes
   * 
   * Se conecta con: stateReconciler (StateReconcilerService.reconcileAll)
   */
  async reconcileAllStates(): Promise<{
    total: number;
    reconciled: number;
    errors: number;
  }> {
    this.logger.log('Starting full state reconciliation...');
    const result = await this.stateReconciler.reconcileAll();
    this.logger.log(
      `Reconciliation complete: ${result.reconciled}/${result.total} reconciled, ${result.errors} errors`,
    );
    return result;
  }

  /**
   * Reconcilia eventos no procesados
   * 
   * Se conecta con: stateReconciler (StateReconcilerService.reconcileUnprocessedEvents)
   */
  async reconcileUnprocessedEvents(): Promise<{
    total: number;
    processed: number;
    errors: number;
  }> {
    this.logger.log('Starting unprocessed events reconciliation...');
    const result = await this.stateReconciler.reconcileUnprocessedEvents();
    this.logger.log(
      `Unprocessed events reconciliation complete: ${result.processed}/${result.total} processed, ${result.errors} errors`,
    );
    return result;
  }

  /**
   * Detiene la subscripción a eventos
   */
  async stopListening(): Promise<void> {
    if (!this.isListening) {
      return;
    }

    try {
      if (this.contract) {
        // Remover todos los listeners
        this.contract.removeAllListeners();
        this.contract = null;
      }

      this.listeners.clear();
      this.isListening = false;
      this.logger.log('Blockchain listener stopped');
    } catch (error) {
      this.logger.error(`Error stopping listener: ${error.message}`, error.stack);
    }
  }

  /**
   * Verifica si el listener está activo
   */
  isActive(): boolean {
    return this.isListening;
  }

  /**
   * Obtiene el estado del listener
   */
  getStatus(): {
    isListening: boolean;
    contractAddress: string | null;
    listenersCount: number;
  } {
    return {
      isListening: this.isListening,
      contractAddress: this.blockchainProvider.escrowContractAddress || null,
      listenersCount: this.listeners.size,
    };
  }
}

