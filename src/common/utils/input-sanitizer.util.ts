/**
 * Input Sanitizer Utility
 * 
 * Utilidades para sanitizar y validar inputs del usuario
 */
export class InputSanitizer {
  /**
   * Sanitiza un string eliminando caracteres peligrosos
   */
  static sanitizeString(input: string): string {
    if (typeof input !== 'string') {
      return '';
    }

    return input
      .trim()
      .replace(/[<>]/g, '') // Eliminar < y >
      .replace(/javascript:/gi, '') // Eliminar javascript:
      .replace(/on\w+=/gi, '') // Eliminar event handlers
      .substring(0, 10000); // Limitar longitud
  }

  /**
   * Sanitiza un número
   */
  static sanitizeNumber(input: any): number | null {
    if (typeof input === 'number') {
      return isNaN(input) ? null : input;
    }

    if (typeof input === 'string') {
      const num = parseFloat(input);
      return isNaN(num) ? null : num;
    }

    return null;
  }

  /**
   * Valida y sanitiza un email
   */
  static sanitizeEmail(input: string): string | null {
    if (typeof input !== 'string') {
      return null;
    }

    const email = input.trim().toLowerCase();
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

    if (!emailRegex.test(email)) {
      return null;
    }

    return email.substring(0, 255);
  }

  /**
   * Valida y sanitiza una dirección de wallet
   */
  static sanitizeWalletAddress(input: string): string | null {
    if (typeof input !== 'string') {
      return null;
    }

    const address = input.trim();
    
    // Validar formato de dirección Ethereum (0x seguido de 40 caracteres hex)
    const ethAddressRegex = /^0x[a-fA-F0-9]{40}$/;
    
    if (!ethAddressRegex.test(address)) {
      return null;
    }

    return address;
  }

  /**
   * Sanitiza un objeto recursivamente
   */
  static sanitizeObject(obj: any): any {
    if (obj === null || obj === undefined) {
      return obj;
    }

    if (typeof obj === 'string') {
      return this.sanitizeString(obj);
    }

    if (typeof obj === 'number') {
      return this.sanitizeNumber(obj);
    }

    if (Array.isArray(obj)) {
      return obj.map((item) => this.sanitizeObject(item));
    }

    if (typeof obj === 'object') {
      const sanitized: any = {};
      for (const key in obj) {
        if (obj.hasOwnProperty(key)) {
          sanitized[key] = this.sanitizeObject(obj[key]);
        }
      }
      return sanitized;
    }

    return obj;
  }

  /**
   * Valida que un string no esté vacío después de sanitizar
   */
  static isValidString(input: string, minLength: number = 1): boolean {
    const sanitized = this.sanitizeString(input);
    return sanitized.length >= minLength;
  }

  /**
   * Valida que un número esté en un rango válido
   */
  static isValidNumber(
    input: any,
    min: number = Number.MIN_SAFE_INTEGER,
    max: number = Number.MAX_SAFE_INTEGER,
  ): boolean {
    const num = this.sanitizeNumber(input);
    if (num === null) {
      return false;
    }
    return num >= min && num <= max;
  }

  /**
   * Valida que una cantidad de crypto sea válida
   */
  static isValidCryptoAmount(input: any): boolean {
    const num = this.sanitizeNumber(input);
    if (num === null) {
      return false;
    }
    return num > 0 && num <= 1000000000; // Máximo 1 billón
  }

  /**
   * Valida que un precio sea válido
   */
  static isValidPrice(input: any): boolean {
    const num = this.sanitizeNumber(input);
    if (num === null) {
      return false;
    }
    return num > 0 && num <= 1000000000;
  }
}

