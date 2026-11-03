use askama::Template;
use axum::{
    Router,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use sqlx::PgPool;
use tower_http::trace::TraceLayer;

pub mod api;
pub mod config;
pub mod db;
pub mod error;
pub mod franchisees;
pub mod setup;
pub mod stores;
pub mod tls;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/", get(setup::index))
        .route("/franchisees", post(setup::register_franchisee))
        .route("/stores", post(setup::open_store))
        .route(
            "/api/franchisees",
            get(api::list_franchisees).post(api::create_franchisee),
        )
        .route("/api/stores", get(api::list_stores).post(api::create_store))
        .route("/health", get(api::health))
        .route("/ready", get(api::ready))
        .route("/static/app.css", get(api::stylesheet))
        .fallback(not_found)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[derive(Template)]
#[template(path = "not_found.html")]
struct NotFoundPage;

impl IntoResponse for NotFoundPage {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(page) => (StatusCode::NOT_FOUND, Html(page)).into_response(),
            Err(error) => {
                tracing::error!(error = %error, "could not render the not-found page");
                (StatusCode::NOT_FOUND, "Page not found").into_response()
            }
        }
    }
}

async fn not_found() -> NotFoundPage {
    NotFoundPage
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode, header},
    };
    use http_body_util::BodyExt;
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

    use super::*;

    fn test_app() -> Router {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://store:store@127.0.0.1:5432/store")
            .expect("the test database URL is valid");
        app(AppState { pool })
    }

    async fn body_text(response: axum::response::Response) -> String {
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("the response body can be read")
            .to_bytes();
        String::from_utf8(bytes.to_vec()).expect("the response body is UTF-8")
    }

    #[tokio::test]
    async fn serves_the_embedded_stylesheet() {
        let response = test_app()
            .oneshot(
                Request::builder()
                    .uri("/static/app.css")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/css; charset=utf-8"
        );
        assert!(body_text(response).await.contains("--ink"));
    }

    #[tokio::test]
    async fn liveness_does_not_need_the_database() {
        let response = test_app()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(body_text(response).await.contains(r#""status":"ok""#));
    }

    #[tokio::test]
    async fn unknown_routes_get_the_html_not_found_page() {
        let response = test_app()
            .oneshot(
                Request::builder()
                    .uri("/no-such-page")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(body_text(response).await.contains("Page not found"));
    }
}
