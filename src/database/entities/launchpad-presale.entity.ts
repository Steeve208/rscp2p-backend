import {
  Entity,
  PrimaryGeneratedColumn,
  Column,
  CreateDateColumn,
  UpdateDateColumn,
  Index,
} from 'typeorm';

@Entity('launchpad_presales')
@Index(['contractAddress'], { unique: true })
export class LaunchpadPresale {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column({ name: 'project_name' })
  projectName: string;

  @Column({ type: 'text', name: 'project_description' })
  projectDescription: string;

  @Column({ name: 'project_icon' })
  projectIcon: string;

  @Column({ default: false, name: 'is_verified' })
  isVerified: boolean;

  @Column({ name: 'contract_address' })
  contractAddress: string;

  @Column({ name: 'token_symbol' })
  tokenSymbol: string;

  @Column('decimal', { precision: 18, scale: 8, name: 'exchange_rate' })
  exchangeRate: number;

  @Column('decimal', { precision: 18, scale: 8, name: 'min_buy' })
  minBuy: number;

  @Column('decimal', { precision: 18, scale: 8, name: 'max_buy' })
  maxBuy: number;

  @Column({ type: 'timestamp', name: 'end_date' })
  endDate: Date;

  @Column('decimal', { precision: 18, scale: 8, name: 'soft_cap' })
  softCap: number;

  @Column('decimal', { precision: 18, scale: 8, name: 'hard_cap' })
  hardCap: number;

  @Column('decimal', { precision: 18, scale: 8, name: 'min_contrib' })
  minContrib: number;

  @Column('decimal', { precision: 18, scale: 8, name: 'max_contrib' })
  maxContrib: number;

  @Column({ type: 'jsonb', name: 'vesting_terms' })
  vestingTerms: {
    tgeUnlock: string;
    cliffPeriod: string;
    linearVesting: string;
    totalMonths?: number;
  };

  @Column({ nullable: true, name: 'audit_url' })
  auditUrl: string;

  @Column({ nullable: true, name: 'contract_url' })
  contractUrl: string;

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;

  @UpdateDateColumn({ name: 'updated_at' })
  updatedAt: Date;
}
