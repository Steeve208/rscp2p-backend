import {
  IsString,
  IsNumber,
  IsNotEmpty,
  IsOptional,
  Min,
  Matches,
} from 'class-validator';

export class CreateEscrowDto {
  @IsString()
  @IsNotEmpty()
  orderId: string;

  @IsString()
  @IsNotEmpty()
  @Matches(/^0x[a-fA-F0-9]+$/, {
    message: 'escrowId must be a valid blockchain address or ID',
  })
  escrowId: string;

  @IsString()
  @IsNotEmpty()
  @Matches(/^0x[a-fA-F0-9]{40}$/, {
    message: 'contractAddress must be a valid Ethereum address',
  })
  contractAddress: string;

  @IsNumber()
  @Min(0.00000001)
  @IsNotEmpty()
  cryptoAmount: number;

  @IsString()
  @IsNotEmpty()
  cryptoCurrency: string;

  @IsString()
  @IsOptional()
  @Matches(/^0x[a-fA-F0-9]{64}$/, {
    message: 'createTransactionHash must be a valid transaction hash',
  })
  createTransactionHash?: string;
}
