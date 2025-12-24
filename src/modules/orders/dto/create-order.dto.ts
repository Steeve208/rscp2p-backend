import {
  IsString,
  IsNumber,
  IsNotEmpty,
  IsOptional,
  Min,
  IsEnum,
  IsDateString,
} from 'class-validator';

export class CreateOrderDto {
  @IsNumber()
  @Min(0.00000001)
  @IsNotEmpty()
  cryptoAmount: number;

  @IsString()
  @IsNotEmpty()
  cryptoCurrency: string;

  @IsNumber()
  @Min(0.01)
  @IsNotEmpty()
  fiatAmount: number;

  @IsString()
  @IsNotEmpty()
  fiatCurrency: string;

  @IsNumber()
  @Min(0)
  @IsOptional()
  pricePerUnit?: number;

  @IsString()
  @IsOptional()
  paymentMethod?: string;

  @IsString()
  @IsOptional()
  terms?: string;

  @IsDateString()
  @IsOptional()
  expiresAt?: string;
}
