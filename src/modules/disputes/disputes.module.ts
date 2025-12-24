import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { DisputesController } from './disputes.controller';
import { DisputesService } from './disputes.service';
import { Dispute } from '../../database/entities/dispute.entity';
import { DisputeEvidence } from '../../database/entities/dispute-evidence.entity';
import { Order } from '../../database/entities/order.entity';
import { User } from '../../database/entities/user.entity';
import { ReputationModule } from '../reputation/reputation.module';
import { EscrowModule } from '../escrow/escrow.module';
import { AuthModule } from '../auth/auth.module';

@Module({
  imports: [
    TypeOrmModule.forFeature([Dispute, DisputeEvidence, Order, User]),
    ReputationModule,
    EscrowModule,
    AuthModule,
  ],
  controllers: [DisputesController],
  providers: [DisputesService],
  exports: [DisputesService],
})
export class DisputesModule {}

