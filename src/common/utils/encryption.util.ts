import * as crypto from 'crypto';

export class EncryptionUtil {
  private static readonly algorithm = 'aes-256-gcm';
  private static readonly keyLength = 32;
  private static readonly ivLength = 16;
  private static readonly saltLength = 64;
  private static readonly tagLength = 16;
  private static readonly tagPosition = this.saltLength + this.ivLength;
  private static readonly encryptedPosition = this.tagPosition + this.tagLength;

  static encrypt(text: string, masterKey: string): string {
    const key = crypto.scryptSync(masterKey, 'salt', this.keyLength);
    const iv = crypto.randomBytes(this.ivLength);
    const salt = crypto.randomBytes(this.saltLength);

    const cipher = crypto.createCipheriv(this.algorithm, key, iv);

    const encrypted = Buffer.concat([
      cipher.update(text, 'utf8'),
      cipher.final(),
    ]);

    const tag = cipher.getAuthTag();

    return Buffer.concat([salt, iv, tag, encrypted]).toString('base64');
  }

  static decrypt(encryptedData: string, masterKey: string): string {
    const data = Buffer.from(encryptedData, 'base64');
    const salt = data.subarray(0, this.saltLength);
    const iv = data.subarray(this.saltLength, this.tagPosition);
    const tag = data.subarray(this.tagPosition, this.encryptedPosition);
    const encrypted = data.subarray(this.encryptedPosition);

    const key = crypto.scryptSync(masterKey, 'salt', this.keyLength);

    const decipher = crypto.createDecipheriv(this.algorithm, key, iv);
    decipher.setAuthTag(tag);

    return decipher.update(encrypted) + decipher.final('utf8');
  }
}

