import { IsString, IsNotEmpty, MinLength, IsOptional, IsDateString } from 'class-validator';

export class CreateDisputeDto {
  @IsString()
  @IsNotEmpty()
  orderId: string;

  @IsString()
  @IsNotEmpty()
  @MinLength(10)
  reason: string;

  @IsDateString()
  @IsOptional()
  responseDeadline?: string;

  @IsDateString()
  @IsOptional()
  evidenceDeadline?: string;
}
