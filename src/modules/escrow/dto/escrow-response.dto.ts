import { Expose } from 'class-transformer';
import { EscrowStatus } from '../../../common/enums/escrow-status.enum';

export class EscrowResponseDto {
  @Expose()
  id: string;

  @Expose({ name: 'order_id' })
  orderId: string;

  @Expose({ name: 'escrow_id' })
  escrowId: string;

  @Expose({ name: 'contract_address' })
  contractAddress: string;

  @Expose({ name: 'create_transaction_hash' })
  createTransactionHash: string;

  @Expose({ name: 'crypto_amount' })
  cryptoAmount: number;

  @Expose({ name: 'crypto_currency' })
  cryptoCurrency: string;

  @Expose()
  status: EscrowStatus;

  @Expose({ name: 'release_transaction_hash' })
  releaseTransactionHash: string;

  @Expose({ name: 'refund_transaction_hash' })
  refundTransactionHash: string;

  @Expose({ name: 'locked_at' })
  lockedAt: Date;

  @Expose({ name: 'released_at' })
  releasedAt: Date;

  @Expose({ name: 'refunded_at' })
  refundedAt: Date;

  @Expose({ name: 'validation_errors' })
  validationErrors: string;

  @Expose({ name: 'created_at' })
  createdAt: Date;

  @Expose({ name: 'updated_at' })
  updatedAt: Date;
}
