use axum::{Json, http::StatusCode, response::IntoResponse};
use datahub_persistence_pg::RepositoryError;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug)]
pub(crate) struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
    details: Option<Value>,
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
}

impl ApiError {
    pub(crate) fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "bad_request", message)
    }

    pub(crate) fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "valid authentication is required",
        )
    }

    pub(crate) fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden", message)
    }

    pub(crate) fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, "conflict", message)
    }

    pub(crate) fn validation(details: Value) -> Self {
        let mut error = Self::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "submitted document is invalid",
        );
        error.details = Some(details);
        error
    }

    pub(crate) fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", message)
    }

    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            details: None,
        }
    }
}

impl From<RepositoryError> for ApiError {
    fn from(error: RepositoryError) -> Self {
        match error {
            RepositoryError::NotFound => {
                Self::new(StatusCode::NOT_FOUND, "not_found", "record was not found")
            }
            RepositoryError::Conflict => Self::conflict("the resource changed or already exists"),
            RepositoryError::Database(_) => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "database_error",
                "database operation failed",
            ),
            RepositoryError::InvalidDocument(_) | RepositoryError::InvalidRole => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "stored_data_invalid",
                "stored data could not be decoded",
            ),
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(_: sqlx::Error) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "database_error",
            "database operation failed",
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status,
            Json(ErrorBody {
                code: self.code,
                message: self.message,
                details: self.details,
            }),
        )
            .into_response()
    }
}
