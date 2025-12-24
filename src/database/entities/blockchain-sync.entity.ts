import {
  Entity,
  PrimaryGeneratedColumn,
  Column,
  CreateDateColumn,
  UpdateDateColumn,
} from 'typeorm';

@Entity('blockchain_sync')
export class BlockchainSync {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column({ name: 'last_synced_block', default: 0 })
  lastSyncedBlock: number;

  @Column({ name: 'last_synced_block_hash' })
  lastSyncedBlockHash: string;

  @Column({ name: 'sync_status', default: 'ACTIVE' })
  syncStatus: 'ACTIVE' | 'PAUSED' | 'ERROR' | 'RESYNCING';

  @Column({ nullable: true, name: 'last_sync_at' })
  lastSyncAt: Date;

  @Column({ nullable: true, name: 'last_error' })
  lastError: string;

  @Column({ default: 0, name: 'total_events_processed' })
  totalEventsProcessed: number;

  @Column({ default: 0, name: 'total_errors' })
  totalErrors: number;

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;

  @UpdateDateColumn({ name: 'updated_at' })
  updatedAt: Date;
}
