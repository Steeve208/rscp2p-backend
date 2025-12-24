export class ValidationResultDto {
  isValid: boolean;
  errors: string[];
  warnings: string[];
  orderId: string;
  escrowId: string;
}
