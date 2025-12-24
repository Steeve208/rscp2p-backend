import {
  Controller,
  Get,
  Post,
  Put,
  Param,
  HttpCode,
  HttpStatus,
  ParseIntPipe,
} from '@nestjs/common';
import { BlockchainService } from './blockchain.service';

@Controller('blockchain')
export class BlockchainController {
  constructor(private readonly blockchainService: BlockchainService) {}

  /**
   * Obtiene el estado de la blockchain
   * GET /api/blockchain/status
   */
  @Get('status')
  @HttpCode(HttpStatus.OK)
  async getStatus() {
    return this.blockchainService.getStatus();
  }

  /**
   * Inicia la sincronización
   * POST /api/blockchain/sync/start
   */
  @Post('sync/start')
  @HttpCode(HttpStatus.OK)
  async startSync() {
    await this.blockchainService.startSync();
    return { message: 'Sincronización iniciada' };
  }

  /**
   * Detiene la sincronización
   * POST /api/blockchain/sync/stop
   */
  @Post('sync/stop')
  @HttpCode(HttpStatus.OK)
  async stopSync() {
    await this.blockchainService.stopSync();
    return { message: 'Sincronización detenida' };
  }

  /**
   * Re-sincroniza desde un bloque específico
   * POST /api/blockchain/sync/resync/:blockNumber
   */
  @Post('sync/resync/:blockNumber')
  @HttpCode(HttpStatus.OK)
  async resyncFromBlock(@Param('blockNumber', ParseIntPipe) blockNumber: number) {
    await this.blockchainService.resyncFromBlock(blockNumber);
    return {
      message: `Re-sincronización iniciada desde bloque ${blockNumber}`,
    };
  }

  /**
   * Re-sincroniza automáticamente si es necesario
   * POST /api/blockchain/sync/auto-resync
   */
  @Post('sync/auto-resync')
  @HttpCode(HttpStatus.OK)
  async autoResync() {
    await this.blockchainService.autoResyncIfNeeded();
    return { message: 'Re-sincronización automática ejecutada' };
  }

  /**
   * Reconcilia todos los estados
   * POST /api/blockchain/reconcile/all
   */
  @Post('reconcile/all')
  @HttpCode(HttpStatus.OK)
  async reconcileAll() {
    return this.blockchainService.reconcileAll();
  }

  /**
   * Reconcilia un escrow específico
   * POST /api/blockchain/reconcile/escrow/:escrowId
   */
  @Post('reconcile/escrow/:escrowId')
  @HttpCode(HttpStatus.OK)
  async reconcileEscrow(@Param('escrowId') escrowId: string) {
    return this.blockchainService.reconcileEscrow(escrowId);
  }

  /**
   * Valida un bloque
   * GET /api/blockchain/validate/block/:blockNumber
   */
  @Get('validate/block/:blockNumber')
  @HttpCode(HttpStatus.OK)
  async validateBlock(@Param('blockNumber', ParseIntPipe) blockNumber: number) {
    return this.blockchainService.validateBlock(blockNumber);
  }

  /**
   * Obtiene el último bloque
   * GET /api/blockchain/latest-block
   */
  @Get('latest-block')
  @HttpCode(HttpStatus.OK)
  async getLatestBlock() {
    const block = await this.blockchainService.getLatestBlock();
    return {
      number: block.number,
      hash: block.hash,
      timestamp: block.timestamp,
      parentHash: block.parentHash,
      transactions: block.transactions.length,
    };
  }

  /**
   * Obtiene el balance de una dirección
   * GET /api/blockchain/balance/:address
   */
  @Get('balance/:address')
  @HttpCode(HttpStatus.OK)
  async getBalance(@Param('address') address: string) {
    return this.blockchainService.getBalance(address);
  }
}