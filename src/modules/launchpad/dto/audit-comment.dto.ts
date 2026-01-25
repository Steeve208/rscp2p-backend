import { IsString, IsNotEmpty } from 'class-validator';

export class AuditCommentDto {
  @IsString()
  @IsNotEmpty()
  text: string;
}
