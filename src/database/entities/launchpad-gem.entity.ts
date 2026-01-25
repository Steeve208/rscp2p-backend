import {
  Entity,
  PrimaryGeneratedColumn,
  Column,
  CreateDateColumn,
  UpdateDateColumn,
  Index,
} from 'typeorm';

@Entity('launchpad_gems')
@Index(['contractAddress'], { unique: true })
export class LaunchpadGem {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column({ name: 'project_icon' })
  projectIcon: string;

  @Column({ name: 'project_name' })
  projectName: string;

  @Column({ type: 'text' })
  description: string;

  @Column({ type: 'int', name: 'security_score', default: 0 })
  securityScore: number;

  @Column('decimal', { precision: 12, scale: 4, name: 'price_change', default: 0 })
  priceChange: number;

  @Column('decimal', { precision: 18, scale: 2, name: 'liquidity_value', default: 0 })
  liquidityValue: number;

  @Column({ name: 'liquidity_currency', default: 'USD' })
  liquidityCurrency: string;

  @Column({ type: 'bigint', name: 'upvotes_number', default: 0 })
  upvotesNumber: number;

  @Column({ type: 'timestamp', name: 'launch_date', nullable: true })
  launchDate: Date;

  @Column({ type: 'jsonb', name: 'sparkline_data', default: [] })
  sparklineData: number[];

  @Column({ name: 'contract_address' })
  contractAddress: string;

  @Column({ nullable: true })
  category: string;

  @Column({ default: false, name: 'is_verified' })
  isVerified: boolean;

  @Column({ default: false, name: 'rug_checked' })
  rugChecked: boolean;

  @Column('decimal', { precision: 18, scale: 8, nullable: true })
  price: number;

  @Column('decimal', { precision: 18, scale: 2, nullable: true, name: 'volume_24h' })
  volume24h: number;

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;

  @UpdateDateColumn({ name: 'updated_at' })
  updatedAt: Date;
}
