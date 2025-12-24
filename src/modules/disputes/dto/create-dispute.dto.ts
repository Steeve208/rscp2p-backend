import { IsString, IsNotEmpty, MinLength } from 'class-validator';

export class CreateDisputeDto {
  @IsString()
  @IsNotEmpty()
  orderId: string;

  @IsString()
  @IsNotEmpty()
  @MinLength(10)
  reason: string;
}
