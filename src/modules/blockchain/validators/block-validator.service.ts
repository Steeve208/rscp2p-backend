import { Injectable, Logger, Inject } from '@nestjs/common';
import { ethers } from 'ethers';

@Injectable()
export class BlockValidatorService {
  private readonly logger = new Logger(BlockValidatorService.name);

  constructor(
    @Inject('BLOCKCHAIN_PROVIDER')
    private readonly blockchainProvider: any,
  ) {}

  /**
   * Valida un bloque
   */
  async validateBlock(blockNumber: number): Promise<{
    isValid: boolean;
    block: ethers.Block;
    errors: string[];
  }> {
    const errors: string[] = [];

    try {
      const block = await this.blockchainProvider.provider.getBlock(blockNumber);

      if (!block) {
        errors.push(`Block ${blockNumber} not found`);
        return { isValid: false, block: null, errors };
      }

      // Validar que el bloque tiene hash
      if (!block.hash) {
        errors.push(`Block ${blockNumber} has no hash`);
      }

      // Validar que el bloque tiene número correcto
      if (block.number !== blockNumber) {
        errors.push(
          `Block number mismatch: expected ${blockNumber}, got ${block.number}`,
        );
      }

      // Validar que el bloque tiene timestamp
      if (!block.timestamp) {
        errors.push(`Block ${blockNumber} has no timestamp`);
      }

      // Validar que el bloque tiene transacciones
      if (block.transactions.length === 0) {
        this.logger.debug(`Block ${blockNumber} has no transactions`);
      }

      // Validar hash del bloque padre (si no es el primer bloque)
      if (blockNumber > 0 && !block.parentHash) {
        errors.push(`Block ${blockNumber} has no parent hash`);
      }

      const isValid = errors.length === 0;

      if (!isValid) {
        this.logger.warn(
          `Block ${blockNumber} validation failed: ${errors.join(', ')}`,
        );
      }

      return { isValid, block, errors };
    } catch (error) {
      this.logger.error(
        `Error validating block ${blockNumber}: ${error.message}`,
        error.stack,
      );
      return {
        isValid: false,
        block: null,
        errors: [error.message],
      };
    }
  }

  /**
   * Valida una cadena de bloques
   */
  async validateBlockChain(
    fromBlock: number,
    toBlock: number,
  ): Promise<{
    isValid: boolean;
    validatedBlocks: number;
    errors: string[];
  }> {
    const errors: string[] = [];
    let validatedBlocks = 0;
    let previousHash: string = null;

    for (let blockNumber = fromBlock; blockNumber <= toBlock; blockNumber++) {
      const validation = await this.validateBlock(blockNumber);

      if (!validation.isValid) {
        errors.push(...validation.errors);
        continue;
      }

      // Validar cadena (hash del bloque anterior)
      if (previousHash && validation.block.parentHash !== previousHash) {
        errors.push(
          `Block chain broken at block ${blockNumber}: parent hash mismatch`,
        );
      }

      previousHash = validation.block.hash;
      validatedBlocks++;
    }

    const isValid = errors.length === 0;

    this.logger.log(
      `Validated ${validatedBlocks} blocks from ${fromBlock} to ${toBlock}. Valid: ${isValid}`,
    );

    return { isValid, validatedBlocks, errors };
  }

  /**
   * Obtiene el último bloque confirmado
   */
  async getLatestBlock(): Promise<ethers.Block> {
    try {
      return await this.blockchainProvider.provider.getBlock('latest');
    } catch (error) {
      this.logger.error(`Error getting latest block: ${error.message}`, error.stack);
      throw error;
    }
  }

  /**
   * Obtiene el número del último bloque
   */
  async getLatestBlockNumber(): Promise<number> {
    const block = await this.getLatestBlock();
    return block.number;
  }
}
