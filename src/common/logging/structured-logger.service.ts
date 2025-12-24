import { Injectable, LoggerService, LogLevel } from '@nestjs/common';
import { ConfigService } from '@nestjs/config';

/**
 * Structured Logger Service
 * 
 * Proporciona logging estructurado en formato JSON para producción
 * y logging legible para desarrollo
 */
@Injectable()
export class StructuredLoggerService implements LoggerService {
  private readonly isProduction: boolean;
  private readonly logLevel: LogLevel;

  constructor(private readonly configService: ConfigService) {
    this.isProduction =
      this.configService.get<string>('app.nodeEnv') === 'production';
    this.logLevel =
      (this.configService.get<string>('LOG_LEVEL') as LogLevel) || 'log';
  }

  /**
   * Log con contexto estructurado
   */
  log(message: string, context?: string, metadata?: Record<string, any>) {
    this.writeLog('log', message, context, metadata);
  }

  error(
    message: string,
    trace?: string,
    context?: string,
    metadata?: Record<string, any>,
  ) {
    this.writeLog('error', message, context, { ...metadata, trace });
  }

  warn(message: string, context?: string, metadata?: Record<string, any>) {
    this.writeLog('warn', message, context, metadata);
  }

  debug(message: string, context?: string, metadata?: Record<string, any>) {
    this.writeLog('debug', message, context, metadata);
  }

  verbose(message: string, context?: string, metadata?: Record<string, any>) {
    this.writeLog('verbose', message, context, metadata);
  }

  /**
   * Escribe el log en formato estructurado o legible
   */
  private writeLog(
    level: LogLevel,
    message: string,
    context?: string,
    metadata?: Record<string, any>,
  ) {
    // Verificar si el nivel de log está habilitado
    if (!this.shouldLog(level)) {
      return;
    }

    const logEntry = {
      timestamp: new Date().toISOString(),
      level: level.toUpperCase(),
      service: 'rsc-backend',
      context: context || 'Application',
      message,
      ...(metadata && Object.keys(metadata).length > 0 && { metadata }),
      ...(process.env.NODE_ENV !== 'production' && {
        pid: process.pid,
        uptime: process.uptime(),
      }),
    };

    if (this.isProduction) {
      // En producción, usar JSON estructurado
      console.log(JSON.stringify(logEntry));
    } else {
      // En desarrollo, usar formato legible
      const contextStr = context ? `[${context}] ` : '';
      const metadataStr = metadata
        ? ` ${JSON.stringify(metadata, null, 2)}`
        : '';
      const logMessage = `${logEntry.timestamp} ${level.toUpperCase()} ${contextStr}${message}${metadataStr}`;
      
      // Usar console apropiado según el nivel
      switch (level) {
        case 'error':
          console.error(logMessage);
          break;
        case 'warn':
          console.warn(logMessage);
          break;
        case 'debug':
        case 'verbose':
          console.debug(logMessage);
          break;
        default:
          console.log(logMessage);
      }
    }
  }

  /**
   * Verifica si debe loguear según el nivel configurado
   */
  private shouldLog(level: LogLevel): boolean {
    const levels: LogLevel[] = ['verbose', 'debug', 'log', 'warn', 'error'];
    const currentLevelIndex = levels.indexOf(this.logLevel);
    const messageLevelIndex = levels.indexOf(level);

    return messageLevelIndex >= currentLevelIndex;
  }

  /**
   * Crea un logger con contexto predefinido
   */
  createContextLogger(context: string) {
    return {
      log: (message: string, metadata?: Record<string, any>) =>
        this.log(message, context, metadata),
      error: (
        message: string,
        trace?: string,
        metadata?: Record<string, any>,
      ) => this.error(message, trace, context, metadata),
      warn: (message: string, metadata?: Record<string, any>) =>
        this.warn(message, context, metadata),
      debug: (message: string, metadata?: Record<string, any>) =>
        this.debug(message, context, metadata),
      verbose: (message: string, metadata?: Record<string, any>) =>
        this.verbose(message, context, metadata),
    };
  }
}

