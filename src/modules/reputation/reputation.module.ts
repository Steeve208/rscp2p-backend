import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { ReputationController } from './reputation.controller';
import { ReputationService } from './reputation.service';
import { User } from '../../database/entities/user.entity';
import { ReputationEvent } from '../../database/entities/reputation-event.entity';
import { Order } from '../../database/entities/order.entity';

@Module({
  imports: [TypeOrmModule.forFeature([User, ReputationEvent, Order])],
  controllers: [ReputationController],
  providers: [ReputationService],
  exports: [ReputationService],
})
export class ReputationModule {}

