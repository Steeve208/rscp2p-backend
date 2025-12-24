import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { BlockchainSyncJob } from './blockchain-sync.job';
import { CleanupJob } from './cleanup.job';
import { ConsistencyCheckJob } from './consistency-check.job';
import { JobTrackerService } from './job-tracker.service';
import { Order } from '../database/entities/order.entity';
import { Notification } from '../database/entities/notification.entity';
import { Escrow } from '../database/entities/escrow.entity';
import { Dispute } from '../database/entities/dispute.entity';
import { BlockchainModule } from '../modules/blockchain/blockchain.module';
import { OrdersModule } from '../modules/orders/orders.module';
import { NotificationsModule } from '../modules/notifications/notifications.module';
import { EscrowModule } from '../modules/escrow/escrow.module';
import { DatabaseModule } from '../database/database.module';
import { JobRecoveryService } from './job-recovery.service';

@Module({
  imports: [
    TypeOrmModule.forFeature([Order, Notification, Escrow, Dispute]),
    DatabaseModule,
    BlockchainModule,
    OrdersModule,
    NotificationsModule,
    EscrowModule,
  ],
  providers: [
    JobTrackerService,
    JobRecoveryService,
    BlockchainSyncJob,
    CleanupJob,
    ConsistencyCheckJob,
  ],
  exports: [JobTrackerService],
})
export class JobsModule {}
