import { Expose, Type } from 'class-transformer';
import { OrderStatus } from '../../../common/enums/order-status.enum';

export class OrderResponseDto {
  @Expose()
  id: string;

  @Expose({ name: 'seller_id' })
  sellerId: string;

  @Expose({ name: 'buyer_id' })
  buyerId: string;

  @Expose({ name: 'crypto_amount' })
  cryptoAmount: number;

  @Expose({ name: 'crypto_currency' })
  cryptoCurrency: string;

  @Expose({ name: 'fiat_amount' })
  fiatAmount: number;

  @Expose({ name: 'fiat_currency' })
  fiatCurrency: string;

  @Expose({ name: 'price_per_unit' })
  pricePerUnit: number;

  @Expose()
  status: OrderStatus;

  @Expose({ name: 'escrow_id' })
  escrowId: string;

  @Expose({ name: 'payment_method' })
  paymentMethod: string;

  @Expose()
  terms: string;

  @Expose({ name: 'expires_at' })
  expiresAt: Date;

  @Expose({ name: 'accepted_at' })
  acceptedAt: Date;

  @Expose({ name: 'completed_at' })
  completedAt: Date;

  @Expose({ name: 'cancelled_at' })
  cancelledAt: Date;

  @Expose({ name: 'cancelled_by' })
  cancelledBy: string;

  @Expose({ name: 'created_at' })
  createdAt: Date;

  @Expose({ name: 'updated_at' })
  updatedAt: Date;
}
