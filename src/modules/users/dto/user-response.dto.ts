import { Expose } from 'class-transformer';

/**
 * DTO para respuesta pública de usuario
 * Solo expone datos públicos, nunca información personal
 */
export class UserResponseDto {
  @Expose()
  id: string;

  @Expose({ name: 'wallet_address' })
  walletAddress: string;

  @Expose({ name: 'reputation_score' })
  reputationScore: number;

  @Expose({ name: 'created_at' })
  createdAt: Date;
}
