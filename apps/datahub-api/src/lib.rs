use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use datahub_kernel::HealthPayload;
use sqlx::PgPool;
use tower_http::trace::TraceLayer;

const SERVICE: &str = "datahub-api";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone)]
struct AppState {
    pool: PgPool,
}

pub fn router(pool: PgPool) -> Router {
    Router::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
        .layer(TraceLayer::new_for_http())
        .with_state(AppState { pool })
}

async fn live() -> Json<HealthPayload> {
    Json(HealthPayload::healthy(SERVICE, VERSION))
}

async fn ready(State(state): State<AppState>) -> (StatusCode, Json<HealthPayload>) {
    match datahub_persistence_pg::check(&state.pool).await {
        Ok(()) => (
            StatusCode::OK,
            Json(HealthPayload::healthy(SERVICE, VERSION)),
        ),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthPayload::unavailable(SERVICE, VERSION)),
        ),
    }
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
    }
}
