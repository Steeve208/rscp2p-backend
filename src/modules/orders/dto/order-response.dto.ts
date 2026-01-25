import { Expose } from 'class-transformer';
import { OrderStatus } from '../../../common/enums/order-status.enum';

export class OrderResponseDto {
  @Expose()
  id: string;

  @Expose()
  sellerId: string;

  @Expose()
  buyerId: string | null;

  @Expose()
  seller?: { id: string; wallet_address: string; reputation_score: number };

  @Expose()
  buyer?: { id: string; wallet_address: string; reputation_score: number } | null;

  @Expose()
  cryptoAmount: string;

  @Expose()
  cryptoCurrency: string;

  @Expose()
  fiatAmount: string;

  @Expose()
  fiatCurrency: string;

  @Expose()
  pricePerUnit: string | null;

  @Expose()
  status: OrderStatus;

  @Expose()
  escrowId: string | null;

  @Expose()
  paymentMethod: string | null;

  @Expose()
  terms: string | null;

  @Expose()
  expiresAt: Date | null;

  @Expose()
  acceptedAt: Date | null;

  @Expose()
  completedAt: Date | null;

  @Expose()
  cancelledAt: Date | null;

  @Expose()
  cancelledBy: 'SELLER' | 'BUYER' | null;

  @Expose()
  disputedAt: Date | null;

  @Expose()
  createdAt: Date;

  @Expose()
  updatedAt: Date;

  @Expose()
  blockchain?: string;

  @Expose()
  tokenAddress?: string;

  @Expose()
  chainId?: number;
}
