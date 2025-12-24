export class DateUtil {
  /**
   * Obtiene fecha actual
   */
  static now(): Date {
    return new Date();
  }

  /**
   * Agrega días a una fecha
   */
  static addDays(date: Date, days: number): Date {
    const result = new Date(date);
    result.setDate(result.getDate() + days);
    return result;
  }

  /**
   * Agrega horas a una fecha
   */
  static addHours(date: Date, hours: number): Date {
    const result = new Date(date);
    result.setHours(result.getHours() + hours);
    return result;
  }

  /**
   * Agrega minutos a una fecha
   */
  static addMinutes(date: Date, minutes: number): Date {
    const result = new Date(date);
    result.setMinutes(result.getMinutes() + minutes);
    return result;
  }

  /**
   * Agrega segundos a una fecha
   */
  static addSeconds(date: Date, seconds: number): Date {
    const result = new Date(date);
    result.setSeconds(result.getSeconds() + seconds);
    return result;
  }

  /**
   * Verifica si una fecha ha expirado
   */
  static isExpired(date: Date): boolean {
    return date < new Date();
  }

  /**
   * Verifica si una fecha está en el futuro
   */
  static isFuture(date: Date): boolean {
    return date > new Date();
  }

  /**
   * Formatea fecha a ISO string
   */
  static formatISO(date: Date): string {
    return date.toISOString();
  }

  /**
   * Calcula diferencia en milisegundos
   */
  static diffInMs(date1: Date, date2: Date): number {
    return Math.abs(date1.getTime() - date2.getTime());
  }

  /**
   * Calcula diferencia en segundos
   */
  static diffInSeconds(date1: Date, date2: Date): number {
    return Math.floor(this.diffInMs(date1, date2) / 1000);
  }

  /**
   * Calcula diferencia en minutos
   */
  static diffInMinutes(date1: Date, date2: Date): number {
    return Math.floor(this.diffInSeconds(date1, date2) / 60);
  }

  /**
   * Calcula diferencia en horas
   */
  static diffInHours(date1: Date, date2: Date): number {
    return Math.floor(this.diffInMinutes(date1, date2) / 60);
  }

  /**
   * Calcula diferencia en días
   */
  static diffInDays(date1: Date, date2: Date): number {
    return Math.floor(this.diffInHours(date1, date2) / 24);
  }

  /**
   * Formatea fecha a formato legible
   */
  static formatReadable(date: Date): string {
    return date.toLocaleString('es-ES', {
      year: 'numeric',
      month: 'long',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }
}

