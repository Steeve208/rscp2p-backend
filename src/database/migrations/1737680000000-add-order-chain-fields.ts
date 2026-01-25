import { MigrationInterface, QueryRunner } from 'typeorm';

export class AddOrderChainFields1737680000000 implements MigrationInterface {
  name = 'AddOrderChainFields1737680000000';

  public async up(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.query(
      `ALTER TABLE "orders" ADD "blockchain" character varying`,
    );
    await queryRunner.query(
      `ALTER TABLE "orders" ADD "token_address" character varying`,
    );
    await queryRunner.query(
      `ALTER TABLE "orders" ADD "chain_id" integer`,
    );
  }

  public async down(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.query(`ALTER TABLE "orders" DROP COLUMN "chain_id"`);
    await queryRunner.query(`ALTER TABLE "orders" DROP COLUMN "token_address"`);
    await queryRunner.query(`ALTER TABLE "orders" DROP COLUMN "blockchain"`);
  }
}
