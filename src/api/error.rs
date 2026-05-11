//! HTTP error envelope per `web-api.md#R3.2`.
//!
//! Every non-2xx response serializes to:
//! ```json
//! {"error": "<code>", "message": "<human>", "details": null}
//! ```
//!
//! The status code is set via the [`IntoResponse`] impl; the `error`
//! code field is a stable machine-readable string. Constructors:
//! [`ApiError::bad_request`] (400), [`ApiError::not_found`] (404),
//! [`ApiError::hub_error`] (500).

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde_json::Value;

/// HTTP error per `web-api.md#R3.2`.
#[derive(Debug, Serialize)]
pub struct ApiError {
    #[serde(skip)]
    pub status: StatusCode,
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl ApiError {
    #[allow(dead_code)] // mutation endpoints in subsequent slices
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "bad_request", message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", message)
    }

    pub fn hub_error(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "hub_error",
            message,
        )
    }

    fn new(status: StatusCode, code: &str, message: impl Into<String>) -> Self {
        Self {
            status,
            error: code.to_string(),
            message: message.into(),
            details: None,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status;
        let body = Json(serde_json::json!({
            "error": self.error,
            "message": self.message,
            "details": self.details,
        }));
        (status, body).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        Self::hub_error(format!("{err:#}"))
    }
}
