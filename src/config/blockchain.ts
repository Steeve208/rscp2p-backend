import { ethers } from 'ethers';
import { ConfigService } from '@nestjs/config';

/**
 * Configuración blockchain
 * 
 * Rol: Configuración blockchain
 * 
 * Contiene:
 * - RPC endpoints (múltiples redes)
 * - Chain IDs
 * - Address del contrato escrow
 * - ABI del contrato escrow
 * 
 * Se conecta con:
 * - blockchain/ (módulo de blockchain)
 * - escrow/ (módulo de escrow)
 * 
 * ⚠️ REGLA FINAL (NO NEGOCIABLE):
 * 
 * Este backend NUNCA debe mover fondos.
 * El wallet aquí creado es SOLO para lectura (si es necesario).
 * NUNCA usar wallet.send() o wallet.sendTransaction().
 * 
 * El backend solo:
 * - Escucha eventos de blockchain
 * - Valida estados
 * - Gestiona estados off-chain
 * 
 * Las transacciones que mueven fondos deben ser ejecutadas
 * directamente por los usuarios desde sus wallets (frontend).
 */

// ============================================
// CHAIN IDs Y REDES SOPORTADAS
// ============================================

export const SUPPORTED_CHAINS = {
  // Ethereum
  ethereum: {
    chainId: 1,
    name: 'Ethereum Mainnet',
    rpcUrls: [
      'https://eth.llamarpc.com',
      'https://rpc.ankr.com/eth',
      'https://eth-mainnet.public.blastapi.io',
    ],
  },
  sepolia: {
    chainId: 11155111,
    name: 'Sepolia Testnet',
    rpcUrls: [
      'https://rpc.sepolia.org',
      'https://sepolia.infura.io/v3/YOUR_PROJECT_ID',
    ],
  },
  // Polygon
  polygon: {
    chainId: 137,
    name: 'Polygon Mainnet',
    rpcUrls: [
      'https://polygon-rpc.com',
      'https://rpc.ankr.com/polygon',
    ],
  },
  mumbai: {
    chainId: 80001,
    name: 'Polygon Mumbai Testnet',
    rpcUrls: [
      'https://rpc-mumbai.maticvigil.com',
      'https://matic-mumbai.chainstacklabs.com',
    ],
  },
  // BSC
  bsc: {
    chainId: 56,
    name: 'BNB Smart Chain',
    rpcUrls: [
      'https://bsc-dataseed.binance.org',
      'https://bsc-dataseed1.defibit.io',
    ],
  },
  bscTestnet: {
    chainId: 97,
    name: 'BSC Testnet',
    rpcUrls: [
      'https://data-seed-prebsc-1-s1.binance.org:8545',
      'https://data-seed-prebsc-2-s1.binance.org:8545',
    ],
  },
} as const;

export type SupportedChain = keyof typeof SUPPORTED_CHAINS;

// ============================================
// ABI DEL CONTRATO ESCROW
// ============================================

/**
 * ABI completo del contrato Escrow
 * 
 * Incluye:
 * - Eventos (para escuchar)
 * - Funciones de lectura (para validar estados)
 * - NO incluye funciones que muevan fondos (release, refund)
 *   porque el backend NUNCA debe ejecutarlas
 */
export const ESCROW_CONTRACT_ABI = [
  // ============================================
  // EVENTOS (para escuchar)
  // ============================================
  'event EscrowCreated(address indexed escrowId, address indexed seller, address indexed buyer, uint256 amount)',
  'event FundsLocked(address indexed escrowId, uint256 amount)',
  'event FundsReleased(address indexed escrowId, address indexed recipient, uint256 amount)',
  'event FundsRefunded(address indexed escrowId, address indexed recipient, uint256 amount)',
  'event DisputeOpened(address indexed escrowId, address indexed initiator)',
  'event DisputeResolved(address indexed escrowId, address indexed resolver, uint8 resolution)',
  
  // ============================================
  // FUNCIONES DE LECTURA (para validar estados)
  // ============================================
  'function getEscrowState(address escrowId) external view returns (uint8 status, address seller, address buyer, uint256 amount, bool isLocked, bool isDisputed)',
  'function escrows(address escrowId) external view returns (address seller, address buyer, uint256 amount, uint8 status, bool isLocked, bool isDisputed)',
  'function isLocked(address escrowId) external view returns (bool)',
  'function isDisputed(address escrowId) external view returns (bool)',
  'function getAmount(address escrowId) external view returns (uint256)',
  
  // ============================================
  // NOTA: Funciones que mueven fondos NO están aquí
  // ============================================
  // ❌ release(address escrowId) - NO incluida
  // ❌ refund(address escrowId) - NO incluida
  // ❌ resolveDispute(address escrowId, uint8 resolution) - NO incluida
  // 
  // Estas funciones deben ser ejecutadas por los usuarios
  // desde el frontend, NUNCA desde el backend
] as const;

/**
 * ABI solo de eventos (para listeners)
 */
export const ESCROW_EVENTS_ABI = [
  'event EscrowCreated(address indexed escrowId, address indexed seller, address indexed buyer, uint256 amount)',
  'event FundsLocked(address indexed escrowId, uint256 amount)',
  'event FundsReleased(address indexed escrowId, address indexed recipient, uint256 amount)',
  'event FundsRefunded(address indexed escrowId, address indexed recipient, uint256 amount)',
  'event DisputeOpened(address indexed escrowId, address indexed initiator)',
  'event DisputeResolved(address indexed escrowId, address indexed resolver, uint8 resolution)',
] as const;

// ============================================
// CONFIGURACIÓN Y PROVIDER
// ============================================

/**
 * Obtiene la configuración de RPC según la red
 */
export const getRpcUrl = (network: string, customRpcUrl?: string): string => {
  // Si hay una URL personalizada, usarla
  if (customRpcUrl && customRpcUrl !== '') {
    return customRpcUrl;
  }

  // Buscar en redes soportadas
  const chainKey = network.toLowerCase() as SupportedChain;
  const chain = SUPPORTED_CHAINS[chainKey];
  
  if (chain && chain.rpcUrls.length > 0) {
    return chain.rpcUrls[0]; // Usar el primer RPC disponible
  }

  // Fallback a Ethereum mainnet
  return SUPPORTED_CHAINS.ethereum.rpcUrls[0];
};

/**
 * Obtiene el Chain ID según la red
 */
export const getChainId = (network: string): number => {
  const chainKey = network.toLowerCase() as SupportedChain;
  const chain = SUPPORTED_CHAINS[chainKey];
  
  if (chain) {
    return chain.chainId;
  }

  // Fallback a Ethereum mainnet
  return SUPPORTED_CHAINS.ethereum.chainId;
};

/**
 * Crea el provider de blockchain
 * 
 * ⚠️ REGLA FINAL: Solo para lectura, NUNCA para enviar transacciones
 */
export const createBlockchainProvider = (configService: ConfigService) => {
  const blockchainConfig = configService.get('blockchain');
  const network = blockchainConfig?.network || 'mainnet';
  const rpcUrl = getRpcUrl(network, blockchainConfig?.rpcUrl);
  const chainId = getChainId(network);

  // Crear provider
  const provider = new ethers.JsonRpcProvider(rpcUrl);

  // IMPORTANTE: El wallet aquí es SOLO para lectura (si es necesario)
  // NUNCA debe usarse para enviar transacciones
  // Si necesitas un wallet para lectura, úsalo solo para:
  // - Leer balances
  // - Leer estados de contratos
  // - Validar direcciones
  // NUNCA para: send(), sendTransaction(), o cualquier método que mueva fondos
  let wallet: ethers.Wallet | null = null;
  const privateKey = blockchainConfig?.privateKey;
  
  // Solo crear wallet si hay una clave privada válida
  // NOTA: Este wallet NO debe tener fondos en producción
  // Solo se usa para lectura si es absolutamente necesario
  if (privateKey && 
      privateKey !== '' && 
      privateKey !== 'your_private_key_here' &&
      !privateKey.startsWith('0xyour_') &&
      !privateKey.includes('your_private_key')) {
    try {
      wallet = new ethers.Wallet(privateKey, provider);
      // ADVERTENCIA: Este wallet NO debe usarse para enviar transacciones
      // Solo para lectura si es necesario
    } catch (error) {
      console.warn('Invalid private key provided, wallet will not be created:', error.message);
    }
  }

  // Dirección del contrato escrow
  const escrowContractAddress = blockchainConfig?.escrowContractAddress || '';

  return {
    provider, // Provider para leer de blockchain
    wallet, // SOLO para lectura, NUNCA para enviar transacciones
    network,
    chainId,
    rpcUrl,
    escrowContractAddress,
    escrowAbi: ESCROW_CONTRACT_ABI,
    escrowEventsAbi: ESCROW_EVENTS_ABI,
  };
};

/**
 * Factory para crear el provider de blockchain
 */
export const createBlockchainProviderFactory = (configService: ConfigService) => {
  return {
    provide: 'BLOCKCHAIN_PROVIDER',
    useFactory: () => createBlockchainProvider(configService),
  };
};

/**
 * Crea una instancia del contrato Escrow (solo lectura)
 * 
 * ⚠️ REGLA FINAL: Solo para leer estados, NUNCA para ejecutar transacciones
 */
export const createEscrowContract = (
  provider: ethers.Provider,
  contractAddress: string,
): ethers.Contract => {
  if (!contractAddress) {
    throw new Error('Escrow contract address is required');
  }

  // Crear contrato SOLO con funciones de lectura
  const contract = new ethers.Contract(
    contractAddress,
    ESCROW_CONTRACT_ABI,
    provider,
  );

  return contract;
};

/**
 * Valida que una dirección sea válida
 */
export const isValidContractAddress = (address: string): boolean => {
  try {
    return ethers.isAddress(address);
  } catch {
    return false;
  }
};

/**
 * Normaliza una dirección de contrato
 */
export const normalizeContractAddress = (address: string): string => {
  try {
    return ethers.getAddress(address);
  } catch {
    throw new Error(`Invalid contract address: ${address}`);
  }
};
