import {
  Entity,
  PrimaryGeneratedColumn,
  Column,
  CreateDateColumn,
  ManyToOne,
  JoinColumn,
  Index,
} from 'typeorm';
import { LaunchpadAudit } from './launchpad-audit.entity';

@Entity('launchpad_audit_comments')
@Index(['auditId'])
export class LaunchpadAuditComment {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @ManyToOne(() => LaunchpadAudit, { eager: false })
  @JoinColumn({ name: 'audit_id' })
  audit: LaunchpadAudit;

  @Column({ name: 'audit_id' })
  auditId: string;

  @Column()
  author: string;

  @Column({ type: 'text' })
  text: string;

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;
}
