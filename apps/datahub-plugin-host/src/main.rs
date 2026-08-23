use std::env;

use anyhow::{Context, Result};
use axum::{Json, Router, routing::get};
use datahub_kernel::HealthPayload;
use tokio::net::TcpListener;
use tracing::info;

const SERVICE: &str = "datahub-plugin-host";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
    datahub_kernel::init_tracing("datahub_plugin_host=info");
    let bind = env::var("DATAHUB_PLUGIN_HOST_BIND").unwrap_or_else(|_| "0.0.0.0:8081".to_owned());
    let app = Router::new().route(
        "/health/live",
        get(|| async { Json(HealthPayload::healthy(SERVICE, VERSION)) }),
    );
    let listener = TcpListener::bind(&bind)
        .await
        .with_context(|| format!("failed to bind plugin host listener to {bind}"))?;

    info!(%bind, "DataHub plugin host listening");
    axum::serve(listener, app)
        .await
        .context("plugin host server failed")
}
