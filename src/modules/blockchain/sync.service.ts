import { Injectable, Logger, Inject } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { BlockchainSync } from '../../database/entities/blockchain-sync.entity';
import { EventListenerService } from './listeners/event-listener.service';
import { BlockValidatorService } from './validators/block-validator.service';
import { StateReconcilerService } from './reconcilers/state-reconciler.service';

@Injectable()
export class SyncService {
  private readonly logger = new Logger(SyncService.name);
  private isSyncing = false;
  private syncConfig = {
    batchSize: 100, // Bloques por batch
    confirmationBlocks: 12, // Bloques de confirmación
  };

  constructor(
    @InjectRepository(BlockchainSync)
    private readonly syncRepository: Repository<BlockchainSync>,
    @Inject('BLOCKCHAIN_PROVIDER')
    private readonly blockchainProvider: any,
    private readonly eventListener: EventListenerService,
    private readonly blockValidator: BlockValidatorService,
    private readonly stateReconciler: StateReconcilerService,
  ) {}

  /**
   * Inicia la sincronización continua
   */
  async startSync(): Promise<void> {
    if (this.isSyncing) {
      this.logger.warn('Sync already in progress');
      return;
    }

    this.isSyncing = true;

    try {
      // Obtener o crear registro de sincronización
      let syncRecords = await this.syncRepository.find();
      let syncRecord = syncRecords[0];

      if (!syncRecord) {
        syncRecord = this.syncRepository.create({
          lastSyncedBlock: 0,
          lastSyncedBlockHash: '',
          syncStatus: 'ACTIVE',
          totalEventsProcessed: 0,
          totalErrors: 0,
        });
        await this.syncRepository.save(syncRecord);
      }

      // Iniciar listener de eventos
      await this.eventListener.startListening();

      // Sincronizar desde el último bloque
      await this.syncFromBlock(syncRecord.lastSyncedBlock);

      this.logger.log('Blockchain sync started');
    } catch (error) {
      this.logger.error(`Error starting sync: ${error.message}`, error.stack);
      this.isSyncing = false;
      throw error;
    }
  }

  /**
   * Sincroniza desde un bloque específico
   */
  async syncFromBlock(fromBlock: number): Promise<void> {
    try {
      const latestBlock = await this.blockValidator.getLatestBlockNumber();
      const toBlock = Math.min(
        fromBlock + this.syncConfig.batchSize,
        latestBlock - this.syncConfig.confirmationBlocks,
      );

      if (toBlock <= fromBlock) {
        this.logger.debug('No new blocks to sync');
        return;
      }

      this.logger.log(`Syncing blocks ${fromBlock} to ${toBlock}`);

      // Validar cadena de bloques
      const validation = await this.blockValidator.validateBlockChain(
        fromBlock,
        toBlock,
      );

      if (!validation.isValid) {
        this.logger.error(
          `Block chain validation failed: ${validation.errors.join(', ')}`,
        );
        await this.updateSyncStatus('ERROR', validation.errors.join('; '));
        return;
      }

      // Escanear eventos
      const events = await this.eventListener.scanHistoricalEvents(
        fromBlock,
        toBlock,
      );

      // Reconciliar estados
      const reconciliation = await this.stateReconciler.reconcileUnprocessedEvents();

      // Actualizar registro de sincronización
      const syncRecords = await this.syncRepository.find();
      const syncRecord = syncRecords[0];

      if (syncRecord) {
        const latestBlockData = await this.blockValidator.getLatestBlock();
        syncRecord.lastSyncedBlock = toBlock;
        syncRecord.lastSyncedBlockHash = latestBlockData.hash;
        syncRecord.lastSyncAt = new Date();
        syncRecord.totalEventsProcessed += events.length;
        syncRecord.totalErrors += reconciliation.errors;
        syncRecord.syncStatus = 'ACTIVE';
        await this.syncRepository.save(syncRecord);
      }

      this.logger.log(
        `Sync completed: ${events.length} events, ${reconciliation.processed} reconciled`,
      );
    } catch (error) {
      this.logger.error(`Error syncing from block ${fromBlock}: ${error.message}`, error.stack);
      await this.updateSyncStatus('ERROR', error.message);
      throw error;
    }
  }

  /**
   * Re-sincroniza desde un bloque específico (útil si se cayó el servicio)
   */
  async resyncFromBlock(fromBlock: number): Promise<void> {
    this.logger.log(`Starting resync from block ${fromBlock}`);

    try {
      await this.updateSyncStatus('RESYNCING', null);

      const latestBlock = await this.blockValidator.getLatestBlockNumber();
      const totalBlocks = latestBlock - fromBlock;

      this.logger.log(`Resyncing ${totalBlocks} blocks`);

      // Sincronizar en batches
      let currentBlock = fromBlock;
      while (currentBlock < latestBlock - this.syncConfig.confirmationBlocks) {
        const toBlock = Math.min(
          currentBlock + this.syncConfig.batchSize,
          latestBlock - this.syncConfig.confirmationBlocks,
        );

        await this.syncFromBlock(currentBlock);
        currentBlock = toBlock + 1;

        // Pequeña pausa para no sobrecargar
        await new Promise((resolve) => setTimeout(resolve, 100));
      }

      await this.updateSyncStatus('ACTIVE', null);
      this.logger.log('Resync completed');
    } catch (error) {
      this.logger.error(`Error during resync: ${error.message}`, error.stack);
      await this.updateSyncStatus('ERROR', error.message);
      throw error;
    }
  }

  /**
   * Re-sincroniza automáticamente si detecta que se cayó
   */
  async autoResyncIfNeeded(): Promise<void> {
    const syncRecords = await this.syncRepository.find();
    const syncRecord = syncRecords[0];

    if (!syncRecord) {
      // Primera vez, sincronizar desde el bloque actual
      const latestBlock = await this.blockValidator.getLatestBlockNumber();
      await this.resyncFromBlock(Math.max(0, latestBlock - 1000)); // Últimos 1000 bloques
      return;
    }

    // Si el estado es ERROR o no se ha sincronizado en mucho tiempo
    const lastSyncAge = syncRecord.lastSyncAt
      ? Date.now() - syncRecord.lastSyncAt.getTime()
      : Infinity;

    if (
      syncRecord.syncStatus === 'ERROR' ||
      lastSyncAge > 3600000 // 1 hora
    ) {
      this.logger.log('Auto-resync needed, starting...');
      await this.resyncFromBlock(syncRecord.lastSyncedBlock);
    }
  }

  /**
   * Obtiene el estado de sincronización
   */
  async getSyncStatus(): Promise<BlockchainSync> {
    let syncRecords = await this.syncRepository.find();
    let syncRecord = syncRecords[0];

    if (!syncRecord) {
      syncRecord = this.syncRepository.create({
        lastSyncedBlock: 0,
        lastSyncedBlockHash: '',
        syncStatus: 'ACTIVE',
        totalEventsProcessed: 0,
        totalErrors: 0,
      });
      await this.syncRepository.save(syncRecord);
    }

    return syncRecord;
  }

  /**
   * Actualiza el estado de sincronización
   */
  private async updateSyncStatus(
    status: 'ACTIVE' | 'PAUSED' | 'ERROR' | 'RESYNCING',
    error: string | null,
  ): Promise<void> {
    const syncRecords = await this.syncRepository.find();
    const syncRecord = syncRecords[0];

    if (syncRecord) {
      syncRecord.syncStatus = status;
      syncRecord.lastError = error;
      await this.syncRepository.save(syncRecord);
    }
  }

  /**
   * Detiene la sincronización
   */
  async stopSync(): Promise<void> {
    this.isSyncing = false;
    await this.eventListener.stopListening();
    await this.updateSyncStatus('PAUSED', null);
    this.logger.log('Blockchain sync stopped');
  }
}
