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
  total?: number;
  page?: number;
  limit?: number;
  totalPages?: number;
}

@Injectable()
export class TransformInterceptor<T> implements NestInterceptor<T, Response<T>> {
  intercept(context: ExecutionContext, next: CallHandler): Observable<Response<T>> {
    return next.handle().pipe(
      map((data) => {
        // Si el controlador ya responde con el formato esperado, no tocar.
        if (data && typeof data === 'object' && 'data' in data) {
          return data as Response<T>;
        }

        // Si viene de ApiResponseDto, mapear a formato { data } o paginado.
        if (data instanceof ApiResponseDto) {
          const innerData = data.data as any;
          if (
            innerData &&
            typeof innerData === 'object' &&
            'data' in innerData &&
            'total' in innerData
          ) {
            return innerData as Response<T>;
          }
          return { data: innerData } as Response<T>;
        }

        // Respuesta estándar.
        return { data } as Response<T>;
      }),
    );
  }
}

