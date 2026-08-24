mod access;
mod configuration;
mod correlation;
mod id;
mod ir;
mod schema;
mod validation;

pub use access::{ProjectAction, ProjectRole};
pub use configuration::{ConfigurationError, required_secret};
pub use correlation::{current_correlation_id, scope_correlation};
pub use id::{
    AuditEventId, BuildId, CustomTypeId, EnvironmentId, FieldId, OutboxEventId, ProjectId,
    ProjectionPlanId, ReleaseId, RevisionId, RowId, SchemaId, SessionId, TableViewId, UserId,
    VariantId,
};
pub use ir::{
    CompilationTarget, TargetField, TargetIr, TargetType, build_target_ir,
    build_target_ir_for_audience, build_target_ir_set, build_target_ir_set_for_audience,
};
pub use schema::{
    Audience, ConfigRow, ConfigValue, CustomTypeDefinition, EnumVariant, FieldDefinition,
    ReferenceMode, SchemaDefinition, TableDefinition, TargetRule, TypeAst,
};
pub use validation::{ValidationCode, ValidationIssue, validate_row, validate_schema};

use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    let _ = tracing_subscriber::fmt()
        .json()
        .flatten_event(true)
        .with_current_span(true)
        .with_env_filter(filter)
        .try_init();
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
