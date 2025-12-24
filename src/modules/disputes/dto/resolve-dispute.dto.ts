import { IsString, IsNotEmpty, IsOptional } from 'class-validator';

export class ResolveDisputeDto {
  @IsString()
  @IsNotEmpty()
  resolution: string;

  @IsString()
  @IsOptional()
  escrowResolution?: string;
}
