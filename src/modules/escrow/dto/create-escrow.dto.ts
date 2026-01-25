import {
  IsString,
  IsNotEmpty,
  IsOptional,
  IsNumberString,
} from 'class-validator';

/**
 * DTO para crear el mapeo orden ↔ escrow.
 *
 * Sin blockchain (modo off-chain): usar contractAddress='OFF_CHAIN',
 * escrowId=UUID generado en frontend y createTransactionHash opcional.
 * Los cambios de estado (LOCKED, RELEASED, REFUNDED) se hacen con PUT /api/escrow/:id.
 */
export class CreateEscrowDto {
  @IsString()
  @IsNotEmpty()
  orderId: string;

  @IsString()
  @IsNotEmpty()
  escrowId: string;

  /** Dirección del contrato on-chain, o 'OFF_CHAIN' para modo manual/sin blockchain */
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
