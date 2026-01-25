import { IsIn, IsString } from 'class-validator';

export class SentimentVoteDto {
  @IsString()
  @IsIn(['bullish', 'bearish'])
  vote: 'bullish' | 'bearish';
}
