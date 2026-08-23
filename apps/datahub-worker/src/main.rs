use std::{env, time::Duration};

use anyhow::{Context, Result};
use tokio::time;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    datahub_kernel::init_tracing("datahub_worker=info");
    let database_url = env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let pool = datahub_persistence_pg::connect(&database_url)
        .await
        .context("failed to connect to PostgreSQL")?;
    let mut heartbeat = time::interval(Duration::from_secs(30));
    let mut outbox = time::interval(Duration::from_secs(1));

    info!("DataHub worker started");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("shutdown signal received");
                break;
            }
            _ = heartbeat.tick() => {
                if let Err(error) = datahub_persistence_pg::check(&pool).await {
                    error!(%error, "PostgreSQL heartbeat failed");
                }
            }
            _ = outbox.tick() => {
                match datahub_persistence_pg::process_outbox_batch(&pool, 100).await {
                    Ok(processed) if processed > 0 => info!(processed, "outbox events projected"),
                    Ok(_) => {}
                    Err(error) => error!(%error, "outbox projection failed"),
                }
            }
        }
    }
    Ok(())
}
