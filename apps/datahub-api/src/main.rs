use std::env;

use anyhow::{Context, Result};
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    datahub_kernel::init_tracing("datahub_api=info,tower_http=info");

    let database_url = datahub_kernel::required_secret("DATABASE_URL")?;
    let bind = env::var("DATAHUB_API_BIND").unwrap_or_else(|_| "0.0.0.0:8080".to_owned());
    let pool = datahub_persistence_pg::connect(&database_url)
        .await
        .context("failed to connect to PostgreSQL")?;
    let listener = TcpListener::bind(&bind)
        .await
        .with_context(|| format!("failed to bind API listener to {bind}"))?;

    info!(%bind, "DataHub API listening");
    let config = datahub_api::ApiConfig::from_env()?;
    axum::serve(listener, datahub_api::router_with_config(pool, config))
        .await
        .context("API server failed")
}
