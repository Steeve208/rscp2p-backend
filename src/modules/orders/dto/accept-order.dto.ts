import { IsString, IsOptional } from 'class-validator';

export class AcceptOrderDto {
  @IsString()
  @IsOptional()
  paymentMethod?: string;
}
