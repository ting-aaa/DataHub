use serde::Serialize;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HealthPayload {
    pub service: &'static str,
    pub status: &'static str,
    pub version: &'static str,
}

impl HealthPayload {
    #[must_use]
    pub const fn healthy(service: &'static str, version: &'static str) -> Self {
        Self {
            service,
            status: "ok",
            version,
        }
    }

    #[must_use]
    pub const fn unavailable(service: &'static str, version: &'static str) -> Self {
        Self {
            service,
            status: "unavailable",
            version,
        }
    }
}

pub fn init_tracing(default_filter: &str) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

#[cfg(test)]
mod tests {
    use super::HealthPayload;

    #[test]
    fn healthy_payload_is_stable() {
        let payload = HealthPayload::healthy("datahub-api", "0.1.0");
        assert_eq!(payload.status, "ok");
        assert_eq!(payload.service, "datahub-api");
    }
}
