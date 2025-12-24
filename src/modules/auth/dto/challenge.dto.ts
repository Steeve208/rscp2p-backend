import { IsString, IsNotEmpty, Matches } from 'class-validator';

export class ChallengeDto {
  @IsString()
  @IsNotEmpty()
  @Matches(/^0x[a-fA-F0-9]{40}$/, {
    message: 'walletAddress must be a valid Ethereum address',
  })
  walletAddress: string;
}
