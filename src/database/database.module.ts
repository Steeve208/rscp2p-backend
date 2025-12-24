import { Module, Global } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { ConfigModule, ConfigService } from '@nestjs/config';
import { createRedisProvider } from '../config/redis';
import { RedisLockService } from './services/redis-lock.service';
import { RedisSessionService } from './services/redis-session.service';
import { RedisRateLimitService } from './services/redis-rate-limit.service';

@Global()
@Module({
  imports: [ConfigModule],
  providers: [
    {
      ...createRedisProvider(new ConfigService()),
      inject: [ConfigService],
    },
    RedisLockService,
    RedisSessionService,
    RedisRateLimitService,
  ],
  exports: [
    RedisLockService,
    RedisSessionService,
    RedisRateLimitService,
    'REDIS_CLIENT',
  ],
})
export class DatabaseModule {}
