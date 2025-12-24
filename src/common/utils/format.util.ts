export class FormatUtil {
  /**
   * Formatea número a decimal con precisión
   */
  static formatDecimal(value: number, decimals: number = 2): string {
    return value.toFixed(decimals);
  }

  /**
   * Formatea dirección Ethereum (acorta)
   */
  static shortenAddress(address: string, start: number = 6, end: number = 4): string {
    if (!address || address.length < start + end) {
      return address;
    }
    return `${address.substring(0, start)}...${address.substring(address.length - end)}`;
  }

  /**
   * Formatea transaction hash (acorta)
   */
  static shortenHash(hash: string, start: number = 10, end: number = 8): string {
    if (!hash || hash.length < start + end) {
      return hash;
    }
    return `${hash.substring(0, start)}...${hash.substring(hash.length - end)}`;
  }

  /**
   * Formatea número grande (K, M, B)
   */
  static formatLargeNumber(value: number): string {
    if (value >= 1_000_000_000) {
      return `${(value / 1_000_000_000).toFixed(2)}B`;
    }
    if (value >= 1_000_000) {
      return `${(value / 1_000_000).toFixed(2)}M`;
    }
    if (value >= 1_000) {
      return `${(value / 1_000).toFixed(2)}K`;
    }
    return value.toString();
  }

  /**
   * Formatea porcentaje
   */
  static formatPercentage(value: number, decimals: number = 2): string {
    return `${value.toFixed(decimals)}%`;
  }

  /**
   * Formatea moneda
   */
  static formatCurrency(value: number, currency: string = 'USD', decimals: number = 2): string {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency,
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    }).format(value);
  }

  /**
   * Capitaliza primera letra
   */
  static capitalize(str: string): string {
    if (!str) return str;
    return str.charAt(0).toUpperCase() + str.slice(1).toLowerCase();
  }

  /**
   * Convierte snake_case a camelCase
   */
  static toCamelCase(str: string): string {
    return str.replace(/_([a-z])/g, (g) => g[1].toUpperCase());
  }

  /**
   * Convierte camelCase a snake_case
   */
  static toSnakeCase(str: string): string {
    return str.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`);
  }
}
