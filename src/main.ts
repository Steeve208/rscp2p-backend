import { NestFactory } from '@nestjs/core';
import { ValidationPipe, Logger } from '@nestjs/common';
import { ConfigService } from '@nestjs/config';
import { FastifyAdapter, NestFastifyApplication } from '@nestjs/platform-fastify';
import helmet from '@fastify/helmet';
import { HttpExceptionFilter } from './common/filters/http-exception.filter';
import { TransformInterceptor } from './common/interceptors/transform.interceptor';
import { LoggingInterceptor } from './common/interceptors/logging.interceptor';
import { AuditInterceptor } from './common/interceptors/audit.interceptor';
import { AppModule } from './app.module';

/**
 * Punto de entrada del backend
 * 
 * Responsabilidades:
 * - Bootstrap de NestJS/Fastify
 * - Inicialización del servidor HTTP
 * - Configuración de seguridad (CORS, Helmet, rate limit)
 * - Configuración global (pipes, filters, interceptors)
 * 
 * NUNCA debe:
 * - Contener lógica de negocio
 * - Importar módulos de órdenes o blockchain
 * - Ejecutar transacciones blockchain
 */
async function bootstrap() {
  const logger = new Logger('Bootstrap');

  try {
    // Crear aplicación NestJS con Fastify
    const app = await NestFactory.create<NestFastifyApplication>(
      AppModule,
      new FastifyAdapter({
        logger: process.env.NODE_ENV === 'development',
      }),
    );

    const configService = app.get(ConfigService);
    const port = configService.get<number>('app.port') || 3000;
    const nodeEnv = configService.get<string>('NODE_ENV') || 'development';
    const corsOrigin = configService.get<string>('app.corsOrigin') || '*';

    logger.log(`🚀 Starting application in ${nodeEnv} mode...`);

    // ============================================
    // SEGURIDAD
    // ============================================

    // Helmet - Headers de seguridad HTTP
    const fastifyInstance = app.getHttpAdapter().getInstance();
    await fastifyInstance.register(helmet as any, {
      contentSecurityPolicy: nodeEnv === 'production',
      crossOriginEmbedderPolicy: nodeEnv === 'production',
    });
    logger.log('✅ Helmet security headers enabled');

    // CORS - Configuración de origen cruzado
    app.enableCors({
      origin: corsOrigin === '*' ? true : corsOrigin.split(','),
      credentials: true,
      methods: ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'OPTIONS'],
      allowedHeaders: ['Content-Type', 'Authorization'],
    });
    logger.log(`✅ CORS enabled for: ${corsOrigin}`);

    // ============================================
    // VALIDACIÓN GLOBAL
    // ============================================

    // Global Validation Pipe
    app.useGlobalPipes(
      new ValidationPipe({
        whitelist: true, // Elimina propiedades no definidas en DTOs
        forbidNonWhitelisted: true, // Rechaza propiedades no permitidas
        transform: true, // Transforma payloads a instancias de DTOs
        transformOptions: {
          enableImplicitConversion: true, // Convierte tipos automáticamente
        },
        disableErrorMessages: nodeEnv === 'production', // Oculta mensajes de error en producción
      }),
    );
    logger.log('✅ Global validation pipe configured');

    // ============================================
    // FILTROS GLOBALES
    // ============================================

    // Global Exception Filter
    app.useGlobalFilters(new HttpExceptionFilter());
    logger.log('✅ Global exception filter configured');

    // ============================================
    // INTERCEPTORS GLOBALES
    // ============================================

    // Transform Interceptor - Formatea respuestas
    app.useGlobalInterceptors(new TransformInterceptor());
    logger.log('✅ Transform interceptor configured');

    // Audit Interceptor - Registra eventos de auditoría
    const auditInterceptor = app.get(AuditInterceptor);
    app.useGlobalInterceptors(auditInterceptor);
    logger.log('✅ Audit interceptor configured');

    // Logging Interceptor - Registra todas las peticiones
    if (nodeEnv === 'development') {
      app.useGlobalInterceptors(new LoggingInterceptor());
      logger.log('✅ Logging interceptor enabled (development)');
    }

    // ============================================
    // CONFIGURACIÓN GLOBAL
    // ============================================

    // Global prefix para todas las rutas
    app.setGlobalPrefix('api');
    logger.log('✅ Global prefix set to: /api');

    // ============================================
    // INICIALIZACIÓN DEL SERVIDOR
    // ============================================

    await app.listen(port, '0.0.0.0');
    
    logger.log(`🚀 Application is running on: http://localhost:${port}/api`);
    logger.log(`📝 Environment: ${nodeEnv}`);
    logger.log(`🔒 Security: Helmet + CORS enabled`);
    
    if (nodeEnv === 'development') {
      logger.log(`📊 Swagger/API docs: http://localhost:${port}/api`);
    }
  } catch (error) {
    logger.error('❌ Failed to start application', error.stack);
    process.exit(1);
  }
}

// Iniciar aplicación
bootstrap();