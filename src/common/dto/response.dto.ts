import { Expose } from 'class-transformer';

export class ApiResponseDto<T> {
  @Expose()
  success: boolean;

  @Expose()
  data?: T;

  @Expose()
  message?: string;

  @Expose()
  error?: string;

  @Expose()
  timestamp: Date;

  constructor(success: boolean, data?: T, message?: string, error?: string) {
    this.success = success;
    this.data = data;
    this.message = message;
    this.error = error;
    this.timestamp = new Date();
  }

  static success<T>(data: T, message?: string): ApiResponseDto<T> {
    return new ApiResponseDto(true, data, message);
  }

  static error(error: string, message?: string): ApiResponseDto<null> {
    return new ApiResponseDto(false, null, message, error);
  }
}
