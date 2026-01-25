import {
  IsString,
  IsNotEmpty,
  IsOptional,
  IsNumberString,
} from 'class-validator';

export class CreateEscrowDto {
  @IsString()
  @IsNotEmpty()
  orderId: string;

  @IsString()
  @IsNotEmpty()
  escrowId: string;

  @IsString()
  @IsNotEmpty()
  contractAddress: string;

  @IsNumberString()
  @IsNotEmpty()
  cryptoAmount: string;

  @IsString()
  @IsNotEmpty()
  cryptoCurrency: string;

  @IsString()
  @IsOptional()
  createTransactionHash?: string;
}
