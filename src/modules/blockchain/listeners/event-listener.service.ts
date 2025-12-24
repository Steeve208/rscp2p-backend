import { Injectable, Logger, Inject } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { BlockchainEvent } from '../../../database/entities/blockchain-event.entity';
import { ethers } from 'ethers';
import { ConfigService } from '@nestjs/config';
import { ESCROW_EVENTS_ABI } from '../../../config/blockchain';

/**
 * ⚠️ REGLA FINAL: Este servicio SOLO escucha eventos de blockchain.
 * NUNCA ejecuta transacciones que muevan fondos.
 * 
 * Las transacciones son ejecutadas por los usuarios desde el frontend.
 * Este servicio solo reacciona a los eventos emitidos.
 */
@Injectable()
export class EventListenerService {
  private readonly logger = new Logger(EventListenerService.name);
  private listeners: Map<string, ethers.ContractEventPayload[]> = new Map();

  constructor(
    @Inject('BLOCKCHAIN_PROVIDER')
    private readonly blockchainProvider: any,
    @InjectRepository(BlockchainEvent)
    private readonly eventRepository: Repository<BlockchainEvent>,
    private readonly configService: ConfigService,
  ) {}

  /**
   * Inicia el listener de eventos del contrato escrow
   */
  async startListening(): Promise<void> {
    const contractAddress = this.configService.get<string>(
      'blockchain.escrowContractAddress',
    );

    if (!contractAddress) {
      this.logger.warn('No escrow contract address configured, skipping event listener');
      return;
    }

    try {
      // Usar ABI desde configuración
      const contract = new ethers.Contract(
        contractAddress,
        ESCROW_EVENTS_ABI,
        this.blockchainProvider.provider,
      );

      // Listener para EscrowCreated
      contract.on('EscrowCreated', async (escrowId, seller, buyer, amount, event) => {
        await this.handleEvent('EscrowCreated', event, {
          escrowId: escrowId.toString(),
          seller: seller.toString(),
          buyer: buyer.toString(),
          amount: amount.toString(),
        });
      });

      // Listener para FundsLocked
      contract.on('FundsLocked', async (escrowId, amount, event) => {
        await this.handleEvent('FundsLocked', event, {
          escrowId: escrowId.toString(),
          amount: amount.toString(),
        });
      });

      // Listener para FundsReleased
      contract.on('FundsReleased', async (escrowId, recipient, amount, event) => {
        await this.handleEvent('FundsReleased', event, {
          escrowId: escrowId.toString(),
          recipient: recipient.toString(),
          amount: amount.toString(),
        });
      });

      // Listener para FundsRefunded
      contract.on('FundsRefunded', async (escrowId, recipient, amount, event) => {
        await this.handleEvent('FundsRefunded', event, {
          escrowId: escrowId.toString(),
          recipient: recipient.toString(),
          amount: amount.toString(),
        });
      });

      // Listener para DisputeOpened
      contract.on('DisputeOpened', async (escrowId, initiator, event) => {
        await this.handleEvent('DisputeOpened', event, {
          escrowId: escrowId.toString(),
          initiator: initiator.toString(),
        });
      });

      this.logger.log(`Event listener started for contract ${contractAddress}`);
    } catch (error) {
      this.logger.error(`Error starting event listener: ${error.message}`, error.stack);
      throw error;
    }
  }

  /**
   * Guarda un evento en la base de datos
   * 
   * Método público para ser usado por otros servicios (ej. BlockchainListener)
   */
  async saveEvent(
    eventName: string,
    event: ethers.Log,
    eventData: any,
  ): Promise<void> {
    return this.handleEvent(eventName, event, eventData);
  }

  /**
   * Maneja un evento recibido
   */
  private async handleEvent(
    eventName: string,
    event: ethers.Log,
    eventData: any,
  ): Promise<void> {
    try {
      // Verificar si el evento ya fue procesado
      const existing = await this.eventRepository.findOne({
        where: { transactionHash: event.transactionHash },
      });

      if (existing) {
        this.logger.debug(`Event already processed: ${event.transactionHash}`);
        return;
      }

      // Obtener información del bloque
      const block = await event.getBlock();

      // Extraer escrowId de los datos del evento
      const escrowId = eventData.escrowId || eventData[0]?.toString();

      // Guardar evento
      const blockchainEvent = this.eventRepository.create({
        eventName,
        contractAddress: event.address,
        transactionHash: event.transactionHash,
        blockNumber: event.blockNumber,
        blockHash: block.hash,
        eventData,
        escrowId,
        processed: false,
      });

      await this.eventRepository.save(blockchainEvent);

      this.logger.log(
        `Event ${eventName} saved: ${event.transactionHash} at block ${event.blockNumber}`,
      );
    } catch (error) {
      this.logger.error(
        `Error handling event ${eventName}: ${error.message}`,
        error.stack,
      );
    }
  }

  /**
   * Escanea eventos históricos desde un bloque específico
   */
  async scanHistoricalEvents(
    fromBlock: number,
    toBlock: number,
  ): Promise<BlockchainEvent[]> {
    const contractAddress = this.configService.get<string>(
      'blockchain.escrowContractAddress',
    );

    if (!contractAddress) {
      throw new Error('No escrow contract address configured');
    }

    // Usar ABI desde configuración
    const contract = new ethers.Contract(
      contractAddress,
      ESCROW_EVENTS_ABI,
      this.blockchainProvider.provider,
    );

    const events: BlockchainEvent[] = [];

    try {
      // Escanear todos los eventos
      const eventNames = [
        'EscrowCreated',
        'FundsLocked',
        'FundsReleased',
        'FundsRefunded',
        'DisputeOpened',
      ];

      for (const eventName of eventNames) {
        const filter = contract.filters[eventName]();
        const logs = await contract.queryFilter(filter, fromBlock, toBlock);

        for (const log of logs) {
          // Verificar si ya existe
          const existing = await this.eventRepository.findOne({
            where: { transactionHash: log.transactionHash },
          });

          if (!existing) {
            const block = await log.getBlock();
            const parsedLog = contract.interface.parseLog(log);

            // Extraer escrowId de los argumentos
            const escrowId = parsedLog.args.escrowId?.toString() || 
                           parsedLog.args[0]?.toString() || 
                           null;

            const blockchainEvent = this.eventRepository.create({
              eventName,
              contractAddress: log.address,
              transactionHash: log.transactionHash,
              blockNumber: log.blockNumber,
              blockHash: block.hash,
              eventData: parsedLog.args,
              escrowId,
              processed: false,
            });

            await this.eventRepository.save(blockchainEvent);
            events.push(blockchainEvent);
          }
        }
      }

      this.logger.log(
        `Scanned ${events.length} historical events from block ${fromBlock} to ${toBlock}`,
      );
    } catch (error) {
      this.logger.error(
        `Error scanning historical events: ${error.message}`,
        error.stack,
      );
      throw error;
    }

    return events;
  }

  /**
   * Detiene todos los listeners
   */
  async stopListening(): Promise<void> {
    // Limpiar listeners si es necesario
    this.logger.log('Event listeners stopped');
  }
}
