import { IsString, IsNotEmpty, IsOptional, IsEnum } from 'class-validator';

export enum EvidenceType {
  IMAGE = 'IMAGE',
  DOCUMENT = 'DOCUMENT',
  TEXT = 'TEXT',
  LINK = 'LINK',
  VIDEO = 'VIDEO',
  AUDIO = 'AUDIO',
}

export class AddEvidenceDto {
  @IsEnum(EvidenceType)
  @IsNotEmpty()
  evidenceType: EvidenceType;

  @IsString()
  @IsNotEmpty()
  evidenceUrl: string;

  @IsString()
  @IsOptional()
  description?: string;
}
