use askama::Template;
use axum::{
    Json,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("could not render a page: {0}")]
    Template(#[from] askama::Error),
    #[error("database operation failed: {0}")]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!(error = %self, "request failed");

        let (status, message) = match self {
            Self::Database(sqlx::Error::RowNotFound) => (
                StatusCode::NOT_FOUND,
                "We could not find the record you asked for.",
            ),
            Self::Database(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "The store data could not be read. Please try again.",
            ),
            Self::Template(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "The page could not be rendered. Please try again.",
            ),
        };

        (status, Html(ErrorPage { message })).into_response()
    }
}

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorPage {
    pub message: &'static str,
}

impl IntoResponse for ErrorPage {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(page) => Html(page).into_response(),
            Err(error) => {
                tracing::error!(error = %error, "could not render the error page");
                (StatusCode::INTERNAL_SERVER_ERROR, self.message).into_response()
            }
        }
    }
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

#[derive(Serialize)]
struct ApiErrorBody {
    error: String,
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: message.into(),
        }
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            message: message.into(),
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        tracing::error!(error = %error, "API database operation failed");
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "The store data could not be read. Please try again.".to_owned(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ApiErrorBody {
                error: self.message,
            }),
        )
            .into_response()
    }
}
