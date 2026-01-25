import {
  Entity,
  PrimaryGeneratedColumn,
  Column,
  CreateDateColumn,
  UpdateDateColumn,
  Index,
} from 'typeorm';

@Entity('launchpad_audits')
@Index(['contractAddress'], { unique: true })
export class LaunchpadAudit {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column({ name: 'project_icon' })
  projectIcon: string;

  @Column({ name: 'project_name' })
  projectName: string;

  @Column({ name: 'contract_address' })
  contractAddress: string;

  @Column({ name: 'full_address' })
  fullAddress: string;

  @Column()
  network: string;

  @Column({ name: 'audit_completed' })
  auditCompleted: string;

  @Column({ default: false, name: 'is_verified' })
  isVerified: boolean;

  @Column()
  verdict: string;

  @Column({ name: 'risk_level' })
  riskLevel: string;

  @Column({ type: 'int', name: 'trust_score', default: 0 })
  trustScore: number;

  @Column({ type: 'text', name: 'trust_summary' })
  trustSummary: string;

  @Column({ type: 'jsonb', name: 'security_checks', default: [] })
  securityChecks: Array<{
    name: string;
    status: 'PASSED' | 'FAILED' | 'STABLE' | 'WARN';
    description: string;
    tooltip?: string;
  }>;

  @Column({ type: 'jsonb', name: 'vulnerabilities', default: {} })
  vulnerabilities: { critical: number; high: number; medium: number; low: number };

  @Column({ type: 'jsonb', name: 'liquidity_locks', default: {} })
  liquidityLocks: {
    totalLocked: string;
    locks: Array<{
      lockerName: string;
      contractAddress: string;
      amount: string;
      unlocksIn: string;
      unlockDate: string;
      txHash: string;
    }>;
  };

  @Column({ type: 'jsonb', name: 'community_sentiment', default: {} })
  communitySentiment: {
    bullish: number;
    bearish: number;
    upvotes: string;
    watchlists: string;
    comments: Array<{ author: string; reputation?: string; text: string }>;
  };

  @Column({ name: 'token_symbol' })
  tokenSymbol: string;

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;

  @UpdateDateColumn({ name: 'updated_at' })
  updatedAt: Date;
}
