import { Expose } from 'class-transformer';

/**
 * DTO para respuesta pública de usuario
 * Solo expone datos públicos, nunca información personal
 */
export class UserResponseDto {
  @Expose()
  id: string;

  @Expose()
  walletAddress: string;

  @Expose()
  reputationScore: number;

  @Expose()
  isActive: boolean;

  @Expose()
  loginCount: number;

  @Expose()
  lastLoginAt: Date;

  @Expose()
  createdAt: Date;
}
