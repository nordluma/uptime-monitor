use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub enum ApiError {
    SqlError(sqlx::Error),
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        Self::SqlError(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::SqlError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("SQL ERROR: {}", e),
            )
                .into_response(),
        }
    }
}
