import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { ConfigModule, ConfigService } from '@nestjs/config';
import { BlockchainController } from './blockchain.controller';
import { BlockchainService } from './blockchain.service';
import { SyncService } from './sync.service';
import { EventListenerService } from './listeners/event-listener.service';
import { BlockValidatorService } from './validators/block-validator.service';
import { StateReconcilerService } from './reconcilers/state-reconciler.service';
import { BlockchainListener } from './blockchain.listener';
import { createBlockchainProvider } from '../../config/blockchain';
import { BlockchainEvent } from '../../database/entities/blockchain-event.entity';
import { BlockchainSync } from '../../database/entities/blockchain-sync.entity';
import { Escrow } from '../../database/entities/escrow.entity';
import { Order } from '../../database/entities/order.entity';
import { EscrowModule } from '../escrow/escrow.module';

@Module({
  imports: [
    ConfigModule,
    TypeOrmModule.forFeature([
      BlockchainEvent,
      BlockchainSync,
      Escrow,
      Order,
    ]),
    EscrowModule,
  ],
  controllers: [BlockchainController],
  providers: [
    BlockchainService,
    SyncService,
    EventListenerService,
    BlockValidatorService,
    StateReconcilerService,
    BlockchainListener,
    {
      provide: 'BLOCKCHAIN_PROVIDER',
      useFactory: (configService: ConfigService) =>
        createBlockchainProvider(configService),
      inject: [ConfigService],
    },
  ],
  exports: [
    BlockchainService,
    SyncService,
    EventListenerService,
    BlockValidatorService,
    StateReconcilerService,
    BlockchainListener,
    'BLOCKCHAIN_PROVIDER',
  ],
})
export class BlockchainModule {}