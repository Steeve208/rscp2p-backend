import {
  Entity,
  PrimaryGeneratedColumn,
  Column,
  CreateDateColumn,
  UpdateDateColumn,
  ManyToOne,
  JoinColumn,
  Index,
} from 'typeorm';
import { User } from './user.entity';
import { OrderStatus } from '../../common/enums/order-status.enum';

/**
 * Modelo DB de órdenes
 * 
 * Rol: Modelo DB de órdenes
 * 
 * Contiene:
 * - id: Identificador único de la orden
 * - status: Estado de la orden (CREATED, AWAITING_FUNDS, etc.)
 * - buyer: Usuario comprador (relación con User)
 * - seller: Usuario vendedor (relación con User)
 * - token: Token/criptomoneda (cryptoCurrency)
 * - amount: Cantidad de criptomoneda (cryptoAmount)
 * - Campos adicionales para P2P
 * 
 * Se conecta con:
 * - ORM (TypeORM)
 * - User entity (relaciones ManyToOne)
 */
@Entity('orders')
@Index(['sellerId', 'status'])
@Index(['buyerId', 'status'])
@Index(['status', 'expiresAt'])
@Index(['cryptoCurrency', 'fiatCurrency'])
export class Order {
  /**
   * ID único de la orden (UUID)
   */
  @PrimaryGeneratedColumn('uuid')
  id: string;

  /**
   * Vendedor (relación con User)
   */
  @ManyToOne(() => User, { eager: false })
  @JoinColumn({ name: 'seller_id' })
  seller: User;

  @Column({ name: 'seller_id' })
  @Index()
  sellerId: string;

  /**
   * Comprador (relación con User, nullable hasta que se acepte)
   */
  @ManyToOne(() => User, { eager: false, nullable: true })
  @JoinColumn({ name: 'buyer_id' })
  buyer: User;

  @Column({ nullable: true, name: 'buyer_id' })
  @Index()
  buyerId: string;

  /**
   * Token/Criptomoneda (ej: ETH, BTC, USDT)
   */
  @Column({ length: 10, name: 'crypto_currency' })
  cryptoCurrency: string;

  /**
   * Cantidad de criptomoneda (amount)
   */
  @Column('decimal', { precision: 18, scale: 8, name: 'crypto_amount' })
  cryptoAmount: number;

  /**
   * Moneda fiat (ej: USD, EUR)
   */
  @Column({ length: 10, name: 'fiat_currency' })
  fiatCurrency: string;

  /**
   * Cantidad de fiat
   */
  @Column('decimal', { precision: 18, scale: 2, name: 'fiat_amount' })
  fiatAmount: number;

  /**
   * Precio por unidad de token
   */
  @Column('decimal', { precision: 18, scale: 8, nullable: true, name: 'price_per_unit' })
  pricePerUnit: number;

  /**
   * Estado de la orden
   */
  @Column({ type: 'enum', enum: OrderStatus, default: OrderStatus.CREATED, name: 'status' })
  @Index()
  status: OrderStatus;

  /**
   * ID del escrow en blockchain (mapeo order_id ↔ escrow_id)
   */
  @Column({ nullable: true, name: 'escrow_id' })
  escrowId: string;

  /**
   * Método de pago (ej: BANK_TRANSFER, PAYPAL)
   */
  @Column({ nullable: true, name: 'payment_method' })
  paymentMethod: string;

  /**
   * Términos y condiciones de la oferta
   */
  @Column({ type: 'text', nullable: true, name: 'terms' })
  terms: string;

  /**
   * Fecha de expiración de la oferta
   */
  @Column({ nullable: true, name: 'expires_at' })
  expiresAt: Date;

  /**
   * Fecha de aceptación de la orden
   */
  @Column({ nullable: true, name: 'accepted_at' })
  acceptedAt: Date;

  /**
   * Fecha de completación de la orden
   */
  @Column({ nullable: true, name: 'completed_at' })
  completedAt: Date;

  /**
   * Fecha de cancelación de la orden
   */
  @Column({ nullable: true, name: 'cancelled_at' })
  cancelledAt: Date;

  /**
   * Quién canceló la orden (SELLER o BUYER)
   */
  @Column({ nullable: true, name: 'cancelled_by' })
  cancelledBy: 'SELLER' | 'BUYER';

  /**
   * Fecha de apertura de disputa
   */
  @Column({ nullable: true, name: 'disputed_at' })
  disputedAt: Date;

  /**
   * Fecha de creación
   */
  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;

  /**
   * Fecha de última actualización
   */
  @UpdateDateColumn({ name: 'updated_at' })
  updatedAt: Date;
}
