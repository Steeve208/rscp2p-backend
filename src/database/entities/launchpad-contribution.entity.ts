import {
  Entity,
  PrimaryGeneratedColumn,
  Column,
  CreateDateColumn,
  UpdateDateColumn,
  ManyToOne,
  JoinColumn,
  Index,
} from 'typeorm';
import { LaunchpadPresale } from './launchpad-presale.entity';
import { ContributionStatus } from '../../common/enums/launchpad.enum';

@Entity('launchpad_contributions')
@Index(['walletAddress'])
@Index(['txHash'], { unique: true })
export class LaunchpadContribution {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @ManyToOne(() => LaunchpadPresale, { eager: false })
  @JoinColumn({ name: 'presale_id' })
  presale: LaunchpadPresale;

  @Column({ name: 'presale_id' })
  presaleId: string;

  @Column({ name: 'wallet_address' })
  walletAddress: string;

  @Column({ name: 'project_name' })
  projectName: string;

  @Column({ name: 'project_icon' })
  projectIcon: string;

  @Column({ name: 'token_symbol' })
  tokenSymbol: string;

  @Column('decimal', { precision: 18, scale: 8, name: 'contribution_amount' })
  contributionAmount: number;

  @Column('decimal', { precision: 18, scale: 8, name: 'buy_price' })
  buyPrice: number;

  @Column('decimal', { precision: 18, scale: 8, name: 'current_value' })
  currentValue: number;

  @Column({ name: 'growth' })
  growth: string;

  @Column({ name: 'is_loss', default: false })
  isLoss: boolean;

  @Column({ type: 'int', name: 'vesting_progress', default: 0 })
  vestingProgress: number;

  @Column({ name: 'next_unlock', nullable: true })
  nextUnlock: string;

  @Column({ name: 'claimable_amount', nullable: true })
  claimableAmount: string;

  @Column({
    type: 'enum',
    enum: ContributionStatus,
    default: ContributionStatus.ACTIVE,
  })
  status: ContributionStatus;

  @Column({ name: 'tx_hash', nullable: true })
  txHash: string;

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;

  @UpdateDateColumn({ name: 'updated_at' })
  updatedAt: Date;
}
