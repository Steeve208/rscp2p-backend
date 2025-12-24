import { ethers } from 'ethers';

export class ValidationUtil {
  /**
   * Valida dirección Ethereum
   */
  static isValidEthereumAddress(address: string): boolean {
    if (!address || typeof address !== 'string') {
      return false;
    }
    try {
      ethers.getAddress(address);
      return true;
    } catch {
      return /^0x[a-fA-F0-9]{40}$/.test(address);
    }
  }

  /**
   * Normaliza dirección Ethereum (checksum)
   */
  static normalizeEthereumAddress(address: string): string {
    try {
      return ethers.getAddress(address);
    } catch {
      return address.toLowerCase();
    }
  }

  /**
   * Valida email
   */
  static isValidEmail(email: string): boolean {
    if (!email || typeof email !== 'string') {
      return false;
    }
    return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
  }

  /**
   * Sanitiza string (elimina caracteres peligrosos)
   */
  static sanitizeString(input: string): string {
    if (!input || typeof input !== 'string') {
      return '';
    }
    return input.trim().replace(/[<>]/g, '');
  }

  /**
   * Valida transaction hash
   */
  static isValidTransactionHash(hash: string): boolean {
    if (!hash || typeof hash !== 'string') {
      return false;
    }
    return /^0x[a-fA-F0-9]{64}$/.test(hash);
  }

  /**
   * Valida nonce (hex string)
   */
  static isValidNonce(nonce: string): boolean {
    if (!nonce || typeof nonce !== 'string') {
      return false;
    }
    return /^0x[a-fA-F0-9]+$/.test(nonce);
  }

  /**
   * Valida UUID
   */
  static isValidUUID(uuid: string): boolean {
    if (!uuid || typeof uuid !== 'string') {
      return false;
    }
    return /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(uuid);
  }

  /**
   * Valida que un número esté en rango
   */
  static isInRange(value: number, min: number, max: number): boolean {
    return value >= min && value <= max;
  }

  /**
   * Valida que un string no esté vacío
   */
  static isNotEmpty(value: string): boolean {
    return value !== null && value !== undefined && value.trim().length > 0;
  }
}

