import { IsString, IsNotEmpty, IsOptional } from 'class-validator';

export class LaunchpadSubmissionDto {
  @IsString()
  @IsNotEmpty()
  contractAddress: string;

  @IsString()
  @IsNotEmpty()
  network: string;

  @IsString()
  @IsNotEmpty()
  auditReport: string;

  @IsString()
  @IsOptional()
  twitter?: string;

  @IsString()
  @IsOptional()
  telegram?: string;
}
