import {
  IsString,
  IsNotEmpty,
  IsOptional,
  IsInt,
  IsDateString,
  IsNumberString,
} from 'class-validator';

export class CreateOrderDto {
  @IsNumberString()
  @IsNotEmpty()
  cryptoAmount: string;

  @IsString()
  @IsNotEmpty()
  cryptoCurrency: string;

  @IsNumberString()
  @IsNotEmpty()
  fiatAmount: string;

  @IsString()
  @IsNotEmpty()
  fiatCurrency: string;

  @IsNumberString()
  @IsOptional()
  pricePerUnit?: string;

  @IsString()
  @IsOptional()
  paymentMethod?: string;

  @IsString()
  @IsOptional()
  terms?: string;

  @IsDateString()
  @IsOptional()
  expiresAt?: string;

  @IsInt()
  @IsOptional()
  chainId?: number;

  @IsString()
  @IsOptional()
  tokenAddress?: string;

  @IsString()
  @IsOptional()
  blockchain?: string;

  @IsString()
  @IsOptional()
  escrowTxHash?: string;

  @IsString()
  @IsOptional()
  escrowContractAddress?: string;
}
