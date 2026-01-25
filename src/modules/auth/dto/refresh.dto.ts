import { IsString, IsNotEmpty, IsOptional } from 'class-validator';

export class RefreshDto {
  @IsString()
  @IsOptional()
  @IsNotEmpty()
  refreshToken?: string;
}
