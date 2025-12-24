import { IsNumber, Min, Max } from 'class-validator';

export class UpdateReputationDto {
  @IsNumber()
  @Min(-100)
  @Max(100)
  score: number;
}
