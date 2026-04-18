"""
RSC P2P Backend - Configuración desde variables de entorno.
Producción: definir ENVIRONMENT=production, JWT_SECRET seguro (nunca el default) y CORS explícito.
"""

import os

from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    model_config = SettingsConfigDict(
        env_file=".env",
        env_file_encoding="utf-8",
        extra="ignore",
    )

    host: str = "0.0.0.0"
    port: int = 8000
    # Desarrollo local + orígenes típicos del sitio (ajuste CORS_ORIGINS en producción).
    cors_origins: str = (
        "http://localhost:3000,http://127.0.0.1:3000,"
        "http://p2prsc.xyz,https://p2prsc.xyz,"
        "http://www.p2prsc.xyz,https://www.p2prsc.xyz"
    )

    # JWT (auth Face ID: challenge/verify + JWT). En producción JWT_SECRET es obligatorio.
    jwt_secret: str = "change-me-in-production"
    jwt_algorithm: str = "HS256"
    jwt_access_expire_minutes: int = 60
    jwt_refresh_expire_days: int = 7

    database_url: str = "sqlite:///./launchpad.db"
    redis_url: str = "redis://localhost:6379/0"
    rate_limit_window_seconds: int = 60
    rate_limit_max_requests: int = 120
    # Auth: límite para challenge/verify (evitar brute-force; en dev el front puede hacer varias llamadas al conectar)
    rate_limit_auth_window_seconds: int = 60
    rate_limit_auth_max_requests: int = 200
    strict_security_headers: bool = True
    environment: str = "development"
    log_level: str = "INFO"

    # Launchpad admin: lista separada por comas de wallets (0x...) que pueden aprobar envíos.
    admin_wallet_addresses: str = ""

    # Webhook secret para blockchain watcher (POST /api/deposits)
    webhook_secret: str = ""

    # Verificación on-chain
    rpc_url: str = ""
    chain_id: int = 1
    min_confirmations: int = 12

    @property
    def cors_origins_list(self) -> list[str]:
        if self.cors_origins.strip() == "*":
            return ["*"]
        return [o.strip() for o in self.cors_origins.split(",") if o.strip()]

    @property
    def is_production(self) -> bool:
        return (os.environ.get("ENVIRONMENT") or self.environment or "").strip().lower() == "production"

    @property
    def admin_wallet_set(self) -> set[str]:
        return {a.strip().lower() for a in self.admin_wallet_addresses.split(",") if a.strip()}


settings = Settings()
