import {
  Entity,
  PrimaryGeneratedColumn,
  Column,
  CreateDateColumn,
  ManyToOne,
  JoinColumn,
  Index,
} from 'typeorm';
import { Dispute } from './dispute.entity';
import { User } from './user.entity';

@Entity('dispute_evidence')
export class DisputeEvidence {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @ManyToOne(() => Dispute, (dispute) => dispute.evidence, { onDelete: 'CASCADE' })
  @JoinColumn({ name: 'dispute_id' })
  dispute: Dispute;

  @Column({ name: 'dispute_id' })
  @Index()
  disputeId: string;

  @ManyToOne(() => User, { eager: false })
  @JoinColumn({ name: 'submitted_by' })
  submittedBy: User;

  @Column({ name: 'submitted_by' })
  @Index()
  submittedById: string;

  @Column({ type: 'text', name: 'evidence_type' })
  evidenceType: string; // 'IMAGE', 'DOCUMENT', 'TEXT', 'LINK', etc.

  @Column({ type: 'text', name: 'evidence_url' })
  evidenceUrl: string;

  @Column({ type: 'text', nullable: true, name: 'description' })
  description: string;

  @Column({ type: 'text', nullable: true, name: 'metadata' })
  metadata: string;

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;
}
