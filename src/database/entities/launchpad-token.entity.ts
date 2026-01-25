import {
  Entity,
  PrimaryGeneratedColumn,
  Column,
  CreateDateColumn,
  UpdateDateColumn,
  Index,
} from 'typeorm';

@Entity('launchpad_tokens')
@Index(['contractAddress'], { unique: true })
export class LaunchpadToken {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column({ name: 'project_icon' })
  projectIcon: string;

  @Column({ name: 'project_name' })
  projectName: string;

  @Column()
  symbol: string;

  @Column('decimal', { precision: 18, scale: 8, default: 0 })
  price: number;

  @Column('decimal', { precision: 12, scale: 4, name: 'price_change_24h', default: 0 })
  priceChange24h: number;

  @Column({ default: false, name: 'is_verified' })
  isVerified: boolean;

  @Column({ name: 'contract_address' })
  contractAddress: string;

  @Column('decimal', { precision: 18, scale: 8, name: 'exchange_rate', default: 0 })
  exchangeRate: number;

  @Column({ type: 'jsonb', name: 'sparkline_data', default: [] })
  sparklineData: number[];

  @Column({ type: 'jsonb', name: 'tokenomics', default: {} })
  tokenomics: {
    totalSupply: string;
    burned: string;
    devWalletLockDays: number;
  };

  @Column({ type: 'jsonb', name: 'dao_sentiment', default: {} })
  daoSentiment: {
    score: number;
    label: string;
    comments: Array<{ author: string; timestamp: string; text: string }>;
  };

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;

  @UpdateDateColumn({ name: 'updated_at' })
  updatedAt: Date;
}
