import {
  Entity,
  PrimaryGeneratedColumn,
  Column,
  CreateDateColumn,
  UpdateDateColumn,
  Index,
} from 'typeorm';

@Entity('launchpad_featured_gems')
@Index(['contractAddress'], { unique: true })
export class LaunchpadFeaturedGem {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column({ name: 'project_name' })
  projectName: string;

  @Column()
  subtitle: string;

  @Column({ type: 'text' })
  description: string;

  @Column({ type: 'timestamp', name: 'end_time' })
  endTime: Date;

  @Column({ name: 'contract_address' })
  contractAddress: string;

  @Column({ name: 'project_icon', nullable: true })
  projectIcon: string;

  @Column({ nullable: true })
  category: string;

  @Column('decimal', { precision: 18, scale: 2, nullable: true })
  raised: number;

  @Column('decimal', { precision: 18, scale: 2, nullable: true })
  target: number;

  @Column({ type: 'jsonb', default: [], nullable: true })
  participants: Array<{ address: string; amount?: number; timestamp?: string }>;

  @Column({ type: 'int', nullable: true, name: 'watching_count' })
  watchingCount: number;

  @Column({ type: 'int', nullable: true, name: 'trending_rank' })
  trendingRank: number;

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;

  @UpdateDateColumn({ name: 'updated_at' })
  updatedAt: Date;
}
