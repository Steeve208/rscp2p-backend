import { IsString, IsNotEmpty, IsOptional } from 'class-validator';

export class ApplyPenaltyDto {
  @IsString()
  @IsNotEmpty()
  userId: string;

  @IsString()
  @IsNotEmpty()
  reason: string;

  @IsString()
  @IsOptional()
  orderId?: string;

  @IsString()
  @IsOptional()
  disputeId?: string;
}
