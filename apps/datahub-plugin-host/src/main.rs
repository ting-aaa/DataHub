use std::{collections::BTreeMap, env};

use anyhow::{Context, Result};
use axum::{Json, Router, routing::get};
use datahub_kernel::HealthPayload;
use datahub_plugin_host::{PluginPackage, PluginRunRequest, plugin_metrics, run_plugin};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;
use tracing::info;

const SERVICE: &str = "datahub-plugin-host";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
    datahub_kernel::init_tracing("datahub_plugin_host=info");
    let mut arguments = env::args().skip(1);
    if arguments.next().as_deref() == Some("run-package") {
        let root = arguments.next().context("missing plugin package path")?;
        let input = arguments.next().unwrap_or_else(|| "echo".into());
        if arguments.next().is_some() {
            anyhow::bail!("run-package accepts a package path and optional input text");
        }
        let package = PluginPackage::load(root)?;
        let request = PluginRunRequest {
            inputs: BTreeMap::from([("input/data.bin".into(), input.into_bytes())]),
        };
        let output = run_plugin(&package, &request)?;
        println!(
            "{} {} {:x}",
            output.path,
            output.content.len(),
            Sha256::digest(&output.content)
        );
        return Ok(());
    }
    let bind = env::var("DATAHUB_PLUGIN_HOST_BIND").unwrap_or_else(|_| "0.0.0.0:8081".to_owned());
    let app = Router::new()
        .route(
            "/health/live",
            get(|| async { Json(HealthPayload::healthy(SERVICE, VERSION)) }),
        )
        .route(
            "/metrics",
            get(|| async {
                let metrics = plugin_metrics();
                format!(
                    "datahub_plugin_runs_total {}\ndatahub_plugin_traps_total {}\ndatahub_plugin_quota_rejections_total {}\n",
                    metrics.runs, metrics.traps, metrics.quota_rejections
                )
            }),
        );
    let listener = TcpListener::bind(&bind)
        .await
        .with_context(|| format!("failed to bind plugin host listener to {bind}"))?;

    info!(%bind, "DataHub plugin host listening");
    axum::serve(listener, app)
        .await
        .context("plugin host server failed")
}
