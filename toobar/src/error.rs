use axum::{
    http::StatusCode,
    response::{IntoResponse, Response}
};

#[derive(Debug)]
pub struct AppError {
    pub message: String,
    pub status: StatusCode
}

impl AppError {
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status: StatusCode::INTERNAL_SERVER_ERROR
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status: StatusCode::NOT_FOUND
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status: StatusCode::BAD_REQUEST
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("handler error: {}", self.message);
        (self.status, self.message).into_response()
    }
}

impl<E: std::error::Error> From<E> for AppError {
    fn from(err: E) -> Self {
        AppError::internal(err.to_string())
    }
}
