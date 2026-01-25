import {
  Entity,
  PrimaryGeneratedColumn,
  Column,
  CreateDateColumn,
  Index,
  Unique,
} from 'typeorm';

@Entity('launchpad_watchlist')
@Unique(['userId', 'contractAddress'])
export class LaunchpadWatchlist {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column({ name: 'user_id' })
  @Index()
  userId: string;

  @Column({ name: 'contract_address' })
  @Index()
  contractAddress: string;

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;
}
