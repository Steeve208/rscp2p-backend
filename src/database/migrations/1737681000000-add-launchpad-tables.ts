import { MigrationInterface, QueryRunner } from 'typeorm';

export class AddLaunchpadTables1737681000000 implements MigrationInterface {
  name = 'AddLaunchpadTables1737681000000';

  public async up(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.query(
      `CREATE TYPE "launchpad_contribution_status_enum" AS ENUM ('active', 'fully-vested')`,
    );
    await queryRunner.query(
      `CREATE TYPE "launchpad_submission_status_enum" AS ENUM ('pending', 'approved', 'rejected')`,
    );
    await queryRunner.query(
      `CREATE TYPE "launchpad_sentiment_vote_enum" AS ENUM ('bullish', 'bearish')`,
    );

    await queryRunner.query(`
      CREATE TABLE "launchpad_gems" (
        "id" uuid NOT NULL,
        "project_icon" character varying NOT NULL,
        "project_name" character varying NOT NULL,
        "description" text NOT NULL,
        "security_score" integer NOT NULL DEFAULT 0,
        "price_change" numeric(12,4) NOT NULL DEFAULT 0,
        "liquidity_value" numeric(18,2) NOT NULL DEFAULT 0,
        "liquidity_currency" character varying NOT NULL DEFAULT 'USD',
        "upvotes_number" bigint NOT NULL DEFAULT 0,
        "launch_date" TIMESTAMP NULL,
        "sparkline_data" jsonb NOT NULL DEFAULT '[]',
        "contract_address" character varying NOT NULL,
        "category" character varying NULL,
        "is_verified" boolean NOT NULL DEFAULT false,
        "rug_checked" boolean NOT NULL DEFAULT false,
        "price" numeric(18,8) NULL,
        "volume_24h" numeric(18,2) NULL,
        "created_at" TIMESTAMP NOT NULL DEFAULT now(),
        "updated_at" TIMESTAMP NOT NULL DEFAULT now(),
        CONSTRAINT "PK_launchpad_gems_id" PRIMARY KEY ("id"),
        CONSTRAINT "UQ_launchpad_gems_contract_address" UNIQUE ("contract_address")
      )
    `);

    await queryRunner.query(`
      CREATE TABLE "launchpad_featured_gems" (
        "id" uuid NOT NULL,
        "project_name" character varying NOT NULL,
        "subtitle" character varying NOT NULL,
        "description" text NOT NULL,
        "end_time" TIMESTAMP NOT NULL,
        "contract_address" character varying NOT NULL,
        "project_icon" character varying NULL,
        "category" character varying NULL,
        "raised" numeric(18,2) NULL,
        "target" numeric(18,2) NULL,
        "participants" jsonb NULL DEFAULT '[]',
        "watching_count" integer NULL,
        "trending_rank" integer NULL,
        "created_at" TIMESTAMP NOT NULL DEFAULT now(),
        "updated_at" TIMESTAMP NOT NULL DEFAULT now(),
        CONSTRAINT "PK_launchpad_featured_gems_id" PRIMARY KEY ("id"),
        CONSTRAINT "UQ_launchpad_featured_gems_contract_address" UNIQUE ("contract_address")
      )
    `);

    await queryRunner.query(`
      CREATE TABLE "launchpad_presales" (
        "id" uuid NOT NULL,
        "project_name" character varying NOT NULL,
        "project_description" text NOT NULL,
        "project_icon" character varying NOT NULL,
        "is_verified" boolean NOT NULL DEFAULT false,
        "contract_address" character varying NOT NULL,
        "token_symbol" character varying NOT NULL,
        "exchange_rate" numeric(18,8) NOT NULL,
        "min_buy" numeric(18,8) NOT NULL,
        "max_buy" numeric(18,8) NOT NULL,
        "end_date" TIMESTAMP NOT NULL,
        "soft_cap" numeric(18,8) NOT NULL,
        "hard_cap" numeric(18,8) NOT NULL,
        "min_contrib" numeric(18,8) NOT NULL,
        "max_contrib" numeric(18,8) NOT NULL,
        "vesting_terms" jsonb NOT NULL,
        "audit_url" character varying NULL,
        "contract_url" character varying NULL,
        "created_at" TIMESTAMP NOT NULL DEFAULT now(),
        "updated_at" TIMESTAMP NOT NULL DEFAULT now(),
        CONSTRAINT "PK_launchpad_presales_id" PRIMARY KEY ("id"),
        CONSTRAINT "UQ_launchpad_presales_contract_address" UNIQUE ("contract_address")
      )
    `);

    await queryRunner.query(`
      CREATE TABLE "launchpad_contributions" (
        "id" uuid NOT NULL,
        "presale_id" uuid NOT NULL,
        "wallet_address" character varying NOT NULL,
        "project_name" character varying NOT NULL,
        "project_icon" character varying NOT NULL,
        "token_symbol" character varying NOT NULL,
        "contribution_amount" numeric(18,8) NOT NULL,
        "buy_price" numeric(18,8) NOT NULL,
        "current_value" numeric(18,8) NOT NULL,
        "growth" character varying NOT NULL,
        "is_loss" boolean NOT NULL DEFAULT false,
        "vesting_progress" integer NOT NULL DEFAULT 0,
        "next_unlock" character varying NULL,
        "claimable_amount" character varying NULL,
        "status" "launchpad_contribution_status_enum" NOT NULL DEFAULT 'active',
        "tx_hash" character varying NULL,
        "created_at" TIMESTAMP NOT NULL DEFAULT now(),
        "updated_at" TIMESTAMP NOT NULL DEFAULT now(),
        CONSTRAINT "PK_launchpad_contributions_id" PRIMARY KEY ("id"),
        CONSTRAINT "UQ_launchpad_contributions_tx_hash" UNIQUE ("tx_hash")
      )
    `);

    await queryRunner.query(`
      CREATE TABLE "launchpad_tokens" (
        "id" uuid NOT NULL,
        "project_icon" character varying NOT NULL,
        "project_name" character varying NOT NULL,
        "symbol" character varying NOT NULL,
        "price" numeric(18,8) NOT NULL DEFAULT 0,
        "price_change_24h" numeric(12,4) NOT NULL DEFAULT 0,
        "is_verified" boolean NOT NULL DEFAULT false,
        "contract_address" character varying NOT NULL,
        "exchange_rate" numeric(18,8) NOT NULL DEFAULT 0,
        "sparkline_data" jsonb NOT NULL DEFAULT '[]',
        "tokenomics" jsonb NOT NULL DEFAULT '{}',
        "dao_sentiment" jsonb NOT NULL DEFAULT '{}',
        "created_at" TIMESTAMP NOT NULL DEFAULT now(),
        "updated_at" TIMESTAMP NOT NULL DEFAULT now(),
        CONSTRAINT "PK_launchpad_tokens_id" PRIMARY KEY ("id"),
        CONSTRAINT "UQ_launchpad_tokens_contract_address" UNIQUE ("contract_address")
      )
    `);

    await queryRunner.query(`
      CREATE TABLE "launchpad_audits" (
        "id" uuid NOT NULL,
        "project_icon" character varying NOT NULL,
        "project_name" character varying NOT NULL,
        "contract_address" character varying NOT NULL,
        "full_address" character varying NOT NULL,
        "network" character varying NOT NULL,
        "audit_completed" character varying NOT NULL,
        "is_verified" boolean NOT NULL DEFAULT false,
        "verdict" character varying NOT NULL,
        "risk_level" character varying NOT NULL,
        "trust_score" integer NOT NULL DEFAULT 0,
        "trust_summary" text NOT NULL,
        "security_checks" jsonb NOT NULL DEFAULT '[]',
        "vulnerabilities" jsonb NOT NULL DEFAULT '{}',
        "liquidity_locks" jsonb NOT NULL DEFAULT '{}',
        "community_sentiment" jsonb NOT NULL DEFAULT '{}',
        "token_symbol" character varying NOT NULL,
        "created_at" TIMESTAMP NOT NULL DEFAULT now(),
        "updated_at" TIMESTAMP NOT NULL DEFAULT now(),
        CONSTRAINT "PK_launchpad_audits_id" PRIMARY KEY ("id"),
        CONSTRAINT "UQ_launchpad_audits_contract_address" UNIQUE ("contract_address")
      )
    `);

    await queryRunner.query(`
      CREATE TABLE "launchpad_audit_comments" (
        "id" uuid NOT NULL,
        "audit_id" uuid NOT NULL,
        "author" character varying NOT NULL,
        "text" text NOT NULL,
        "created_at" TIMESTAMP NOT NULL DEFAULT now(),
        CONSTRAINT "PK_launchpad_audit_comments_id" PRIMARY KEY ("id")
      )
    `);

    await queryRunner.query(`
      CREATE TABLE "launchpad_watchlist" (
        "id" uuid NOT NULL,
        "user_id" character varying NOT NULL,
        "contract_address" character varying NOT NULL,
        "created_at" TIMESTAMP NOT NULL DEFAULT now(),
        CONSTRAINT "PK_launchpad_watchlist_id" PRIMARY KEY ("id"),
        CONSTRAINT "UQ_launchpad_watchlist_user_contract" UNIQUE ("user_id", "contract_address")
      )
    `);

    await queryRunner.query(`
      CREATE TABLE "launchpad_submissions" (
        "id" uuid NOT NULL,
        "user_id" character varying NOT NULL,
        "contract_address" character varying NOT NULL,
        "network" character varying NOT NULL,
        "audit_report" character varying NOT NULL,
        "twitter" character varying NULL,
        "telegram" character varying NULL,
        "status" "launchpad_submission_status_enum" NOT NULL DEFAULT 'pending',
        "reviewed_at" TIMESTAMP NULL,
        "reviewer_notes" text NULL,
        "created_at" TIMESTAMP NOT NULL DEFAULT now(),
        "updated_at" TIMESTAMP NOT NULL DEFAULT now(),
        CONSTRAINT "PK_launchpad_submissions_id" PRIMARY KEY ("id")
      )
    `);

    await queryRunner.query(`
      CREATE TABLE "launchpad_token_votes" (
        "id" uuid NOT NULL,
        "user_id" character varying NOT NULL,
        "contract_address" character varying NOT NULL,
        "vote" "launchpad_sentiment_vote_enum" NOT NULL,
        "created_at" TIMESTAMP NOT NULL DEFAULT now(),
        "updated_at" TIMESTAMP NOT NULL DEFAULT now(),
        CONSTRAINT "PK_launchpad_token_votes_id" PRIMARY KEY ("id"),
        CONSTRAINT "UQ_launchpad_token_votes_user_contract" UNIQUE ("user_id", "contract_address")
      )
    `);

    await queryRunner.query(
      `CREATE INDEX "IDX_launchpad_contributions_wallet" ON "launchpad_contributions" ("wallet_address")`,
    );
    await queryRunner.query(
      `CREATE INDEX "IDX_launchpad_audit_comments_audit" ON "launchpad_audit_comments" ("audit_id")`,
    );
    await queryRunner.query(
      `CREATE INDEX "IDX_launchpad_watchlist_user" ON "launchpad_watchlist" ("user_id")`,
    );
    await queryRunner.query(
      `CREATE INDEX "IDX_launchpad_token_votes_user" ON "launchpad_token_votes" ("user_id")`,
    );
  }

  public async down(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.query(`DROP INDEX "IDX_launchpad_token_votes_user"`);
    await queryRunner.query(`DROP INDEX "IDX_launchpad_watchlist_user"`);
    await queryRunner.query(`DROP INDEX "IDX_launchpad_audit_comments_audit"`);
    await queryRunner.query(`DROP INDEX "IDX_launchpad_contributions_wallet"`);

    await queryRunner.query(`DROP TABLE "launchpad_token_votes"`);
    await queryRunner.query(`DROP TABLE "launchpad_submissions"`);
    await queryRunner.query(`DROP TABLE "launchpad_watchlist"`);
    await queryRunner.query(`DROP TABLE "launchpad_audit_comments"`);
    await queryRunner.query(`DROP TABLE "launchpad_audits"`);
    await queryRunner.query(`DROP TABLE "launchpad_tokens"`);
    await queryRunner.query(`DROP TABLE "launchpad_contributions"`);
    await queryRunner.query(`DROP TABLE "launchpad_presales"`);
    await queryRunner.query(`DROP TABLE "launchpad_featured_gems"`);
    await queryRunner.query(`DROP TABLE "launchpad_gems"`);

    await queryRunner.query(`DROP TYPE "launchpad_sentiment_vote_enum"`);
    await queryRunner.query(`DROP TYPE "launchpad_submission_status_enum"`);
    await queryRunner.query(`DROP TYPE "launchpad_contribution_status_enum"`);
  }
}
