mod error;
mod routes;

use std::{
    env,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Instant,
};

use anyhow::{Context, Result};
use axum::{
    Json, Router,
    body::Body,
    extract::{Request, State},
    http::{HeaderValue, Response, StatusCode},
    middleware::{self, Next},
    response::IntoResponse,
    routing::get,
};
use datahub_kernel::HealthPayload;
use sqlx::PgPool;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

const SERVICE: &str = "datahub-api";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone)]
pub(crate) struct AppState {
    pool: PgPool,
    config: ApiConfig,
    metrics: Arc<HttpMetrics>,
}

#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub auth_rate_limit: i64,
    pub mutation_rate_limit: i64,
    pub rate_limit_window_seconds: i64,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            auth_rate_limit: 10,
            mutation_rate_limit: 240,
            rate_limit_window_seconds: 60,
        }
    }
}

impl ApiConfig {
    /// Loads bounded API controls from the environment.
    ///
    /// # Errors
    /// Returns an error for zero, negative, or malformed values.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            auth_rate_limit: positive_env("DATAHUB_AUTH_RATE_LIMIT", 10)?,
            mutation_rate_limit: positive_env("DATAHUB_MUTATION_RATE_LIMIT", 240)?,
            rate_limit_window_seconds: positive_env("DATAHUB_RATE_LIMIT_WINDOW_SECONDS", 60)?,
        })
    }
}

fn positive_env(name: &str, default: i64) -> Result<i64> {
    let value = env::var(name)
        .map_or_else(|_| Ok(default), |raw| raw.parse::<i64>())
        .with_context(|| format!("{name} must be a positive integer"))?;
    anyhow::ensure!(value > 0, "{name} must be a positive integer");
    Ok(value)
}

#[derive(Debug, Default)]
struct HttpMetrics {
    requests: AtomicU64,
    responses_2xx: AtomicU64,
    responses_4xx: AtomicU64,
    responses_5xx: AtomicU64,
    latency_le_10ms: AtomicU64,
    latency_le_100ms: AtomicU64,
    latency_le_1000ms: AtomicU64,
    latency_over_1000ms: AtomicU64,
    database_ready: AtomicBool,
}

pub fn router(pool: PgPool) -> Router {
    router_with_config(pool, ApiConfig::default())
}

pub fn router_with_config(pool: PgPool, config: ApiConfig) -> Router {
    let state = AppState {
        pool,
        config,
        metrics: Arc::new(HttpMetrics::default()),
    };
    Router::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
        .route("/metrics", get(metrics))
        .nest("/v1", routes::router())
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<Body>| {
                let request_id = request
                    .extensions()
                    .get::<Uuid>()
                    .copied()
                    .unwrap_or_else(Uuid::nil);
                tracing::info_span!(
                    "http_request",
                    %request_id,
                    method = %request.method(),
                    uri = %request.uri()
                )
            }),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            observe_request,
        ))
        .with_state(state)
}

async fn live() -> Json<HealthPayload> {
    Json(HealthPayload::healthy(SERVICE, VERSION))
}

async fn ready(State(state): State<AppState>) -> (StatusCode, Json<HealthPayload>) {
    if datahub_persistence_pg::check(&state.pool).await.is_ok() {
        state.metrics.database_ready.store(true, Ordering::Relaxed);
        (
            StatusCode::OK,
            Json(HealthPayload::healthy(SERVICE, VERSION)),
        )
    } else {
        state.metrics.database_ready.store(false, Ordering::Relaxed);
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthPayload::unavailable(SERVICE, VERSION)),
        )
    }
}

async fn observe_request(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response<Body> {
    let correlation_id = request
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value).ok())
        .unwrap_or_else(Uuid::now_v7);
    request.extensions_mut().insert(correlation_id);
    let started = Instant::now();
    state.metrics.requests.fetch_add(1, Ordering::Relaxed);
    let mut response = datahub_kernel::scope_correlation(correlation_id, next.run(request)).await;
    let elapsed_ms = started.elapsed().as_millis();
    match response.status().as_u16() {
        200..=299 => state.metrics.responses_2xx.fetch_add(1, Ordering::Relaxed),
        400..=499 => state.metrics.responses_4xx.fetch_add(1, Ordering::Relaxed),
        500..=599 => state.metrics.responses_5xx.fetch_add(1, Ordering::Relaxed),
        _ => 0,
    };
    match elapsed_ms {
        0..=10 => state
            .metrics
            .latency_le_10ms
            .fetch_add(1, Ordering::Relaxed),
        11..=100 => state
            .metrics
            .latency_le_100ms
            .fetch_add(1, Ordering::Relaxed),
        101..=1000 => state
            .metrics
            .latency_le_1000ms
            .fetch_add(1, Ordering::Relaxed),
        _ => state
            .metrics
            .latency_over_1000ms
            .fetch_add(1, Ordering::Relaxed),
    };
    if let Ok(value) = HeaderValue::from_str(&correlation_id.to_string()) {
        response.headers_mut().insert("x-request-id", value);
    }
    response
}

async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    let operational = datahub_persistence_pg::operational_metrics(&state.pool).await;
    let (status, operational) = match operational {
        Ok(metrics) => (StatusCode::OK, metrics),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            datahub_persistence_pg::OperationalMetrics {
                outbox_pending: 0,
                outbox_retrying: 0,
                outbox_dead_lettered: 0,
                checkpoints_failed: 0,
                published_releases: 0,
            },
        ),
    };
    let body = format!(
        "# TYPE datahub_http_requests_total counter\ndatahub_http_requests_total {}\n\
# TYPE datahub_http_responses_total counter\ndatahub_http_responses_total{{class=\"2xx\"}} {}\ndatahub_http_responses_total{{class=\"4xx\"}} {}\ndatahub_http_responses_total{{class=\"5xx\"}} {}\n\
# TYPE datahub_http_latency_bucket counter\ndatahub_http_latency_bucket{{le=\"10\"}} {}\ndatahub_http_latency_bucket{{le=\"100\"}} {}\ndatahub_http_latency_bucket{{le=\"1000\"}} {}\ndatahub_http_latency_bucket{{le=\"+Inf\"}} {}\n\
# TYPE datahub_database_ready gauge\ndatahub_database_ready {}\n\
# TYPE datahub_outbox_events gauge\ndatahub_outbox_events{{state=\"pending\"}} {}\ndatahub_outbox_events{{state=\"retrying\"}} {}\ndatahub_outbox_events{{state=\"dead_lettered\"}} {}\n\
datahub_projection_checkpoints_failed {}\ndatahub_releases_published {}\n",
        state.metrics.requests.load(Ordering::Relaxed),
        state.metrics.responses_2xx.load(Ordering::Relaxed),
        state.metrics.responses_4xx.load(Ordering::Relaxed),
        state.metrics.responses_5xx.load(Ordering::Relaxed),
        state.metrics.latency_le_10ms.load(Ordering::Relaxed),
        state.metrics.latency_le_100ms.load(Ordering::Relaxed),
        state.metrics.latency_le_1000ms.load(Ordering::Relaxed),
        state.metrics.latency_over_1000ms.load(Ordering::Relaxed),
        u8::from(state.metrics.database_ready.load(Ordering::Relaxed)),
        operational.outbox_pending,
        operational.outbox_retrying,
        operational.outbox_dead_lettered,
        operational.checkpoints_failed,
        operational.published_releases,
    );
    (
        status,
        [("content-type", "text/plain; version=0.0.4")],
        body,
    )
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request};
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

    use super::router;

    #[tokio::test]
    async fn liveness_does_not_require_a_database_connection() {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://datahub:unused@localhost/datahub")
            .expect("lazy pool should accept a valid URL");
        let response = router(pool)
            .oneshot(
                Request::builder()
                    .uri("/health/live")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), 200);
        assert!(response.headers().contains_key("x-request-id"));
    }
}
