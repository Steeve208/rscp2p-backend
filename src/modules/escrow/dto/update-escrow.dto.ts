import {
  IsString,
  IsOptional,
  IsEnum,
  IsDateString,
} from 'class-validator';
import { EscrowStatus } from '../../../common/enums/escrow-status.enum';

export class UpdateEscrowDto {
  @IsEnum(EscrowStatus)
  @IsOptional()
  status?: EscrowStatus;

  @IsString()
  @IsOptional()
  releaseTransactionHash?: string;

  @IsString()
  @IsOptional()
  refundTransactionHash?: string;

  @IsString()
  @IsOptional()
  createTransactionHash?: string;

  @IsDateString()
  @IsOptional()
  releasedAt?: string;

  @IsDateString()
  @IsOptional()
  refundedAt?: string;
}
