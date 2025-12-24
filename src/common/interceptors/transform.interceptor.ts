import {
  Injectable,
  NestInterceptor,
  ExecutionContext,
  CallHandler,
} from '@nestjs/common';
import { Observable } from 'rxjs';
import { map } from 'rxjs/operators';
import { ApiResponseDto } from '../dto';

export interface Response<T> {
  data: T;
  statusCode: number;
  message?: string;
  timestamp: string;
}

@Injectable()
export class TransformInterceptor<T> implements NestInterceptor<T, Response<T>> {
  intercept(context: ExecutionContext, next: CallHandler): Observable<Response<T>> {
    const statusCode = context.switchToHttp().getResponse().statusCode;
    
    return next.handle().pipe(
      map((data) => {
        // Si ya es una ApiResponseDto, retornarla tal cual
        if (data instanceof ApiResponseDto) {
          return {
            statusCode,
            ...data,
          } as any;
        }

        // Si es un objeto con paginación, mantener estructura
        if (data && typeof data === 'object' && 'data' in data && 'total' in data) {
          return {
            statusCode,
            data,
            timestamp: new Date().toISOString(),
          };
        }

        // Respuesta estándar
        return {
          statusCode,
          data,
          timestamp: new Date().toISOString(),
        };
      }),
    );
  }
}

