import { Expose } from 'class-transformer';
import { EscrowStatus } from '../../../common/enums/escrow-status.enum';

export class EscrowResponseDto {
  @Expose()
  id: string;

  @Expose()
  orderId: string;

  @Expose()
  escrowId: string;

  @Expose()
  contractAddress: string;

  @Expose()
  createTransactionHash: string | null;

  @Expose()
  cryptoAmount: string;

  @Expose()
  cryptoCurrency: string;

  @Expose()
  status: EscrowStatus;

  @Expose()
  releaseTransactionHash: string | null;

  @Expose()
  refundTransactionHash: string | null;

  @Expose()
  lockedAt: Date | null;

  @Expose()
  releasedAt: Date | null;

  @Expose()
  refundedAt: Date | null;

  @Expose()
  validationErrors: string | null;

  @Expose()
  createdAt: Date;

  @Expose()
  updatedAt: Date;
}
