import {
  Entity,
  PrimaryGeneratedColumn,
  Column,
  CreateDateColumn,
  UpdateDateColumn,
  Index,
  Unique,
} from 'typeorm';
import { SentimentVote } from '../../common/enums/launchpad.enum';

@Entity('launchpad_token_votes')
@Unique(['userId', 'contractAddress'])
export class LaunchpadTokenVote {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column({ name: 'user_id' })
  @Index()
  userId: string;

  @Column({ name: 'contract_address' })
  @Index()
  contractAddress: string;

  @Column({ type: 'enum', enum: SentimentVote })
  vote: SentimentVote;

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;

  @UpdateDateColumn({ name: 'updated_at' })
  updatedAt: Date;
}
