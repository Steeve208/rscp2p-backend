/**
 * Constantes compartidas
 */

export const DEFAULT_PAGE = 1;
export const DEFAULT_LIMIT = 20;
export const MAX_LIMIT = 100;

export const RATE_LIMIT_DEFAULT = {
  MAX_REQUESTS: 100,
  WINDOW_SECONDS: 60,
};

export const JWT_DEFAULT_EXPIRES_IN = '24h';
export const JWT_REFRESH_EXPIRES_IN = '7d';

export const NONCE_TTL_SECONDS = 300; // 5 minutos
export const SESSION_TTL_SECONDS = 86400; // 24 horas
export const REFRESH_TOKEN_TTL_SECONDS = 604800; // 7 días

export const REPUTATION_SCORE_MIN = -100;
export const REPUTATION_SCORE_MAX = 100;

export const DISPUTE_TIMERS = {
  RESPONSE_DEADLINE_HOURS: 48,
  EVIDENCE_DEADLINE_HOURS: 72,
  ESCALATION_DAYS: 7,
};

export const BLOCKCHAIN_SYNC = {
  BATCH_SIZE: 100,
  CONFIRMATION_BLOCKS: 12,
};
