import {
  Entity,
  PrimaryGeneratedColumn,
  Column,
  CreateDateColumn,
  UpdateDateColumn,
  ManyToOne,
  JoinColumn,
  OneToMany,
  Index,
} from 'typeorm';
import { DisputeStatus } from '../../common/enums/dispute-status.enum';
import { Order } from './order.entity';
import { User } from './user.entity';
import { DisputeEvidence } from './dispute-evidence.entity';

@Entity('disputes')
export class Dispute {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @ManyToOne(() => Order, { eager: false })
  @JoinColumn({ name: 'order_id' })
  order: Order;

  @Column({ name: 'order_id' })
  @Index()
  orderId: string;

  @ManyToOne(() => User, { eager: false })
  @JoinColumn({ name: 'initiator_id' })
  initiator: User;

  @Column({ name: 'initiator_id' })
  @Index()
  initiatorId: string;

  @ManyToOne(() => User, { eager: false, nullable: true })
  @JoinColumn({ name: 'respondent_id' })
  respondent: User;

  @Column({ nullable: true, name: 'respondent_id' })
  respondentId: string;

  @Column({ type: 'text', name: 'reason' })
  reason: string;

  @Column({ type: 'enum', enum: DisputeStatus, default: DisputeStatus.OPEN, name: 'status' })
  @Index()
  status: DisputeStatus;

  @Column({ nullable: true, type: 'text', name: 'resolution' })
  resolution: string;

  @Column({ nullable: true, name: 'resolved_by' })
  resolvedBy: string;

  @Column({ nullable: true, name: 'resolved_at' })
  resolvedAt: Date;

  @Column({ nullable: true, name: 'escalated_at' })
  escalatedAt: Date;

  @Column({ nullable: true, name: 'expires_at' })
  expiresAt: Date;

  @Column({ nullable: true, name: 'response_deadline' })
  responseDeadline: Date;

  @Column({ nullable: true, name: 'evidence_deadline' })
  evidenceDeadline: Date;

  @Column({ nullable: true, name: 'escrow_resolution' })
  escrowResolution: string;

  @Column({ nullable: true, name: 'escrow_resolved_at' })
  escrowResolvedAt: Date;

  @OneToMany(() => DisputeEvidence, (evidence) => evidence.dispute, { cascade: true })
  evidence: DisputeEvidence[];

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;

  @UpdateDateColumn({ name: 'updated_at' })
  updatedAt: Date;
}

