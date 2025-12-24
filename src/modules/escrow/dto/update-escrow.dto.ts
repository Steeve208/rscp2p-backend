import {
  IsString,
  IsOptional,
  IsEnum,
  Matches,
} from 'class-validator';
import { EscrowStatus } from '../../../common/enums/escrow-status.enum';

export class UpdateEscrowDto {
  @IsEnum(EscrowStatus)
  @IsOptional()
  status?: EscrowStatus;

  @IsString()
  @IsOptional()
  @Matches(/^0x[a-fA-F0-9]{64}$/, {
    message: 'releaseTransactionHash must be a valid transaction hash',
  })
  releaseTransactionHash?: string;

  @IsString()
  @IsOptional()
  @Matches(/^0x[a-fA-F0-9]{64}$/, {
    message: 'refundTransactionHash must be a valid transaction hash',
  })
  refundTransactionHash?: string;
}
