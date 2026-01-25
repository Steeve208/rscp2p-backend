import { IsString, IsNotEmpty, IsOptional } from 'class-validator';

export class CreatePresaleContributionDto {
  @IsString()
  @IsNotEmpty()
  walletAddress: string;

  @IsString()
  @IsNotEmpty()
  amount: string;

  @IsString()
  @IsOptional()
  txHash?: string;
}
