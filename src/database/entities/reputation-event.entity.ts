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

export enum ReputationEventType {
  TRADE_COMPLETED = 'TRADE_COMPLETED',
  TRADE_CANCELLED = 'TRADE_CANCELLED',
  DISPUTE_OPENED = 'DISPUTE_OPENED',
  DISPUTE_RESOLVED_FAVOR = 'DISPUTE_RESOLVED_FAVOR',
  DISPUTE_RESOLVED_AGAINST = 'DISPUTE_RESOLVED_AGAINST',
  PENALTY = 'PENALTY',
  BONUS = 'BONUS',
}

@Entity('reputation_events')
export class ReputationEvent {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @ManyToOne(() => User, { eager: false })
  @JoinColumn({ name: 'user_id' })
  user: User;

  @Column({ name: 'user_id' })
  @Index()
  userId: string;

  @Column({ type: 'enum', enum: ReputationEventType, name: 'event_type' })
  @Index()
  eventType: ReputationEventType;

  @Column({ type: 'decimal', precision: 10, scale: 2, name: 'score_change' })
  scoreChange: number;

  @Column({ nullable: true, name: 'order_id' })
  @Index()
  orderId: string;

  @Column({ nullable: true, name: 'dispute_id' })
  @Index()
  disputeId: string;

  @Column({ type: 'text', nullable: true, name: 'reason' })
  reason: string;

  @Column({ type: 'text', nullable: true, name: 'metadata' })
  metadata: string;

  @Column({ name: 'previous_score', type: 'decimal', precision: 10, scale: 2 })
  previousScore: number;

  @Column({ name: 'new_score', type: 'decimal', precision: 10, scale: 2 })
  newScore: number;

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;
}
