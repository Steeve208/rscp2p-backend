import {
  Entity,
  PrimaryGeneratedColumn,
  Column,
  CreateDateColumn,
  UpdateDateColumn,
  ManyToOne,
  JoinColumn,
  Index,
  Unique,
} from 'typeorm';
import { EscrowStatus } from '../../common/enums/escrow-status.enum';
import { Order } from './order.entity';

@Entity('escrows')
@Unique(['orderId'])
@Unique(['escrowId'])
export class Escrow {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @ManyToOne(() => Order, { eager: false })
  @JoinColumn({ name: 'order_id' })
  order: Order;

  @Column({ name: 'order_id', unique: true })
  @Index()
  orderId: string;

  @Column({ unique: true, name: 'escrow_id' })
  @Index()
  escrowId: string;

  @Column({ name: 'contract_address' })
  contractAddress: string;

  @Column({ nullable: true, name: 'create_transaction_hash' })
  createTransactionHash: string;

  @Column('decimal', { precision: 18, scale: 8, name: 'crypto_amount' })
  cryptoAmount: number;

  @Column({ length: 10, name: 'crypto_currency' })
  cryptoCurrency: string;

  @Column({ type: 'enum', enum: EscrowStatus, default: EscrowStatus.PENDING, name: 'status' })
  @Index()
  status: EscrowStatus;

  @Column({ nullable: true, name: 'release_transaction_hash' })
  releaseTransactionHash: string;

  @Column({ nullable: true, name: 'refund_transaction_hash' })
  refundTransactionHash: string;

  @Column({ nullable: true, name: 'locked_at' })
  lockedAt: Date;

  @Column({ nullable: true, name: 'released_at' })
  releasedAt: Date;

  @Column({ nullable: true, name: 'refunded_at' })
  refundedAt: Date;

  @Column({ type: 'text', nullable: true, name: 'validation_errors' })
  validationErrors: string;

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;

  @UpdateDateColumn({ name: 'updated_at' })
  updatedAt: Date;
}

