//! RSC Gateway — system bootstrap entry point.
//!
//! This file only starts infrastructure. No business logic belongs here.

use rsc_gateway::internal::config::Config;
use rsc_gateway::internal::observability;
use rsc_gateway::internal::state::AppState;
use rsc_gateway::routes;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("fatal: {err:#}");
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    // 2. Load `.env` for local dev only — production must inject env via K8s/DO/Vault.
    //    Never commit `.env` (see .env.example + .gitignore).
    dotenvy::dotenv().ok();

    // 3. Load and validate configuration (fail fast — never start half-configured)
    let config = Config::load().map_err(|e| anyhow::anyhow!(e))?;
    config.validate().map_err(|e| anyhow::anyhow!(e))?;

    // 1. Structured logging + metrics + OTLP tracing
    observability::init(config.environment, &config.observability)?;

    info!(
        environment = ?config.environment,
        version = env!("CARGO_PKG_VERSION"),
        "RSC Gateway bootstrapping"
    );

    // 4–5. Connect PostgreSQL and Redis
    // 6. Build global AppState (config + pools + HTTP client)
    let state = AppState::build(config).await?;

    // 7–8. Compose API routes and middleware stack
    let app = routes::create_router(state.clone());

    // 9. Bind and serve (0.0.0.0 for external access in DO/K8s)
    let addr = state.config.listen_addr();
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!(%addr, "RSC Gateway listening");

    axum::serve(listener, app.into_make_service())
    .await
    .map_err(|e| {
        error!(error = %e, "server error");
        anyhow::anyhow!(e)
    })?;

    Ok(())
}
