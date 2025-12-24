import {
  Entity,
  PrimaryGeneratedColumn,
  Column,
  CreateDateColumn,
  Index,
} from 'typeorm';

@Entity('blockchain_events')
export class BlockchainEvent {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column({ name: 'event_name' })
  @Index()
  eventName: string;

  @Column({ name: 'contract_address' })
  @Index()
  contractAddress: string;

  @Column({ name: 'transaction_hash', unique: true })
  @Index()
  transactionHash: string;

  @Column({ name: 'block_number' })
  @Index()
  blockNumber: number;

  @Column({ name: 'block_hash' })
  blockHash: string;

  @Column({ type: 'jsonb', name: 'event_data' })
  eventData: any;

  @Column({ name: 'escrow_id', nullable: true })
  @Index()
  escrowId: string;

  @Column({ name: 'order_id', nullable: true })
  @Index()
  orderId: string;

  @Column({ default: false, name: 'processed' })
  @Index()
  processed: boolean;

  @Column({ nullable: true, name: 'processed_at' })
  processedAt: Date;

  @Column({ type: 'text', nullable: true, name: 'error_message' })
  errorMessage: string;

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;
}
