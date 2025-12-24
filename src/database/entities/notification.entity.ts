import {
  Entity,
  PrimaryGeneratedColumn,
  Column,
  CreateDateColumn,
  ManyToOne,
  JoinColumn,
  Index,
} from 'typeorm';
import { User } from './user.entity';

export enum NotificationType {
  ORDER_CREATED = 'ORDER_CREATED',
  ORDER_ACCEPTED = 'ORDER_ACCEPTED',
  ORDER_COMPLETED = 'ORDER_COMPLETED',
  ORDER_CANCELLED = 'ORDER_CANCELLED',
  ORDER_DISPUTED = 'ORDER_DISPUTED',
  DISPUTE_OPENED = 'DISPUTE_OPENED',
  DISPUTE_RESOLVED = 'DISPUTE_RESOLVED',
  ESCROW_LOCKED = 'ESCROW_LOCKED',
  ESCROW_RELEASED = 'ESCROW_RELEASED',
  ESCROW_REFUNDED = 'ESCROW_REFUNDED',
  MARKET_UPDATE = 'MARKET_UPDATE',
  PRICE_UPDATE = 'PRICE_UPDATE',
  REPUTATION_CHANGE = 'REPUTATION_CHANGE',
}

@Entity('notifications')
export class Notification {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @ManyToOne(() => User, { eager: false })
  @JoinColumn({ name: 'user_id' })
  user: User;

  @Column({ name: 'user_id' })
  @Index()
  userId: string;

  @Column({ type: 'enum', enum: NotificationType, name: 'type' })
  @Index()
  type: NotificationType;

  @Column({ type: 'text', name: 'title' })
  title: string;

  @Column({ type: 'text', name: 'message' })
  message: string;

  @Column({ default: false, name: 'read' })
  @Index()
  read: boolean;

  @Column({ nullable: true, name: 'read_at' })
  readAt: Date;

  @Column({ type: 'jsonb', nullable: true, name: 'data' })
  data: any;

  @Column({ nullable: true, name: 'order_id' })
  @Index()
  orderId: string;

  @Column({ nullable: true, name: 'dispute_id' })
  @Index()
  disputeId: string;

  @Column({ nullable: true, name: 'escrow_id' })
  @Index()
  escrowId: string;

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;
}
