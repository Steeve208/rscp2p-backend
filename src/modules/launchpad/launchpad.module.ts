import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { JwtModule } from '@nestjs/jwt';
import { ConfigModule, ConfigService } from '@nestjs/config';
import { LaunchpadController } from './launchpad.controller';
import { LaunchpadService } from './launchpad.service';
import { LaunchpadGateway } from './launchpad.gateway';
import { AuthModule } from '../auth/auth.module';
import { LaunchpadGem } from '../../database/entities/launchpad-gem.entity';
import { LaunchpadFeaturedGem } from '../../database/entities/launchpad-featured-gem.entity';
import { LaunchpadPresale } from '../../database/entities/launchpad-presale.entity';
import { LaunchpadContribution } from '../../database/entities/launchpad-contribution.entity';
import { LaunchpadToken } from '../../database/entities/launchpad-token.entity';
import { LaunchpadAudit } from '../../database/entities/launchpad-audit.entity';
import { LaunchpadAuditComment } from '../../database/entities/launchpad-audit-comment.entity';
import { LaunchpadWatchlist } from '../../database/entities/launchpad-watchlist.entity';
import { LaunchpadSubmission } from '../../database/entities/launchpad-submission.entity';
import { LaunchpadTokenVote } from '../../database/entities/launchpad-token-vote.entity';

@Module({
  imports: [
    AuthModule,
    TypeOrmModule.forFeature([
      LaunchpadGem,
      LaunchpadFeaturedGem,
      LaunchpadPresale,
      LaunchpadContribution,
      LaunchpadToken,
      LaunchpadAudit,
      LaunchpadAuditComment,
      LaunchpadWatchlist,
      LaunchpadSubmission,
      LaunchpadTokenVote,
    ]),
    ConfigModule,
    JwtModule.registerAsync({
      imports: [ConfigModule],
      useFactory: (configService: ConfigService) => ({
        secret: configService.get<string>('jwt.secret'),
      }),
      inject: [ConfigService],
    }),
  ],
  controllers: [LaunchpadController],
  providers: [LaunchpadService, LaunchpadGateway],
  exports: [LaunchpadService, LaunchpadGateway],
})
export class LaunchpadModule {}
