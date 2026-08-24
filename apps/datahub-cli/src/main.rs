use std::env;

use anyhow::{Context, Result};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    datahub_kernel::init_tracing("datahub_cli=info");
    let command = env::args().nth(1).unwrap_or_else(|| "migrate".to_owned());
    if command != "migrate" {
        anyhow::bail!("unsupported command: {command}; expected `migrate`");
    }

    let database_url = datahub_kernel::required_secret("DATABASE_URL")?;
    let pool = datahub_persistence_pg::connect(&database_url)
        .await
        .context("failed to connect to PostgreSQL")?;
    datahub_persistence_pg::migrate(&pool)
        .await
        .context("failed to apply database migrations")?;
    info!("database migrations are current");
    Ok(())
}
