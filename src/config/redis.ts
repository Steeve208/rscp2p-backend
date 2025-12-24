import { Redis } from 'ioredis';
import { ConfigService } from '@nestjs/config';

export const createRedisClient = (configService: ConfigService): Redis => {
  const redisConfig = configService.get('redis') || {};

  return new Redis({
    host: redisConfig.host || 'localhost',
    port: redisConfig.port || 6379,
    password: redisConfig.password || undefined,
    retryStrategy: (times) => {
      const delay = Math.min(times * 50, 2000);
      return delay;
    },
    maxRetriesPerRequest: 3,
    lazyConnect: true, // No conectar automáticamente, permitir que falle silenciosamente si Redis no está disponible
  });
};

export const createRedisProvider = (configService: ConfigService) => {
  return {
    provide: 'REDIS_CLIENT',
    useFactory: () => createRedisClient(configService),
  };
};

