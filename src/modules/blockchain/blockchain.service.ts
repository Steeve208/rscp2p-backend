import { Injectable, Inject, Logger } from '@nestjs/common';
import { ethers } from 'ethers';
import { SyncService } from './sync.service';
import { StateReconcilerService } from './reconcilers/state-reconciler.service';
import { BlockValidatorService } from './validators/block-validator.service';
import { EventListenerService } from './listeners/event-listener.service';

@Injectable()
export class BlockchainService {
  private readonly logger = new Logger(BlockchainService.name);

  constructor(
    @Inject('BLOCKCHAIN_PROVIDER')
    private readonly blockchainProvider: any,
    private readonly syncService: SyncService,
    private readonly stateReconciler: StateReconcilerService,
    private readonly blockValidator: BlockValidatorService,
    private readonly eventListener: EventListenerService,
  ) {}

  /**
   * Obtiene el estado de la conexión blockchain
   */
  async getStatus() {
    try {
      const latestBlock = await this.blockValidator.getLatestBlockNumber();
      const syncStatus = await this.syncService.getSyncStatus();

      return {
        status: 'connected',
        network: this.blockchainProvider.network,
        latestBlock,
        syncStatus: syncStatus.syncStatus,
        lastSyncedBlock: syncStatus.lastSyncedBlock,
        totalEventsProcessed: syncStatus.totalEventsProcessed,
        totalErrors: syncStatus.totalErrors,
      };
    } catch (error) {
      this.logger.error(`Error getting status: ${error.message}`, error.stack);
      return {
        status: 'error',
        error: error.message,
      };
    }
  }

  /**
   * Inicia la sincronización
   */
  async startSync(): Promise<void> {
    await this.syncService.startSync();
  }

  /**
   * Detiene la sincronización
   */
  async stopSync(): Promise<void> {
    await this.syncService.stopSync();
  }

  /**
   * Re-sincroniza desde un bloque específico
   */
  async resyncFromBlock(blockNumber: number): Promise<void> {
    await this.syncService.resyncFromBlock(blockNumber);
  }

  /**
   * Re-sincroniza automáticamente si es necesario
   */
  async autoResyncIfNeeded(): Promise<void> {
    await this.syncService.autoResyncIfNeeded();
  }

  /**
   * Reconcilia todos los estados
   */
  async reconcileAll(): Promise<{
    total: number;
    reconciled: number;
    errors: number;
  }> {
    return this.stateReconciler.reconcileAll();
  }

  /**
   * Reconcilia un escrow específico
   */
  async reconcileEscrow(escrowId: string): Promise<{
    reconciled: boolean;
    changes: string[];
  }> {
    return this.stateReconciler.reconcileEscrow(escrowId);
  }

  /**
   * Valida un bloque
   */
  async validateBlock(blockNumber: number) {
    return this.blockValidator.validateBlock(blockNumber);
  }

  /**
   * Obtiene el último bloque
   */
  async getLatestBlock() {
    return this.blockValidator.getLatestBlock();
  }

  /**
   * Obtiene el balance de una dirección
   */
  async getBalance(address: string) {
    try {
      const balance = await this.blockchainProvider.provider.getBalance(address);
      return {
        address,
        balance: ethers.formatEther(balance),
        balanceWei: balance.toString(),
      };
    } catch (error) {
      this.logger.error(`Error getting balance: ${error.message}`, error.stack);
      throw error;
    }
  }
}