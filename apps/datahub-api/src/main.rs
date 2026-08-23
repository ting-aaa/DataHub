use std::env;

use anyhow::{Context, Result};
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    datahub_kernel::init_tracing("datahub_api=info,tower_http=info");

    let database_url = env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let bind = env::var("DATAHUB_API_BIND").unwrap_or_else(|_| "0.0.0.0:8080".to_owned());
    let pool = datahub_persistence_pg::connect(&database_url)
        .await
        .context("failed to connect to PostgreSQL")?;
    let listener = TcpListener::bind(&bind)
        .await
        .with_context(|| format!("failed to bind API listener to {bind}"))?;

    info!(%bind, "DataHub API listening");
    axum::serve(listener, datahub_api::router(pool))
        .await
        .context("API server failed")
}
