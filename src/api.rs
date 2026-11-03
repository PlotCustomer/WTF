use axum::{
    Json,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AppState,
    error::ApiError,
    franchisees::{self, CreateFranchiseeError, FranchiseeOverview},
    stores::{self, CreateStoreError, StoreOverview},
};

#[derive(Debug, Deserialize)]
pub struct FranchiseePayload {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct StorePayload {
    pub franchisee_id: String,
    pub name: String,
    pub address: String,
}

#[derive(Debug, Serialize)]
pub struct Health {
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<&'static str>,
}

pub async fn list_franchisees(
    State(state): State<AppState>,
) -> Result<Json<Vec<FranchiseeOverview>>, ApiError> {
    Ok(Json(franchisees::list_overview(&state.pool).await?))
}

pub async fn create_franchisee(
    State(state): State<AppState>,
    Json(payload): Json<FranchiseePayload>,
) -> Result<Response, ApiError> {
    let input = franchisees::validate(&payload.name, &payload.email)
        .map_err(|errors| ApiError::bad_request(errors.join(" ")))?;

    match franchisees::create(&state.pool, input).await {
        Ok(franchisee) => Ok(created(franchisee)),
        Err(CreateFranchiseeError::EmailTaken) => Err(ApiError::conflict(
            "A franchisee with that email address already exists.",
        )),
        Err(CreateFranchiseeError::Database(error)) => Err(error.into()),
    }
}

pub async fn list_stores(
    State(state): State<AppState>,
) -> Result<Json<Vec<StoreOverview>>, ApiError> {
    Ok(Json(stores::list_overview(&state.pool).await?))
}

pub async fn create_store(
    State(state): State<AppState>,
    Json(payload): Json<StorePayload>,
) -> Result<Response, ApiError> {
    let franchisee_id = Uuid::parse_str(payload.franchisee_id.trim())
        .map_err(|_| ApiError::bad_request("franchisee_id must be a UUID."))?;

    let input = stores::validate(&payload.name, &payload.address, Some(franchisee_id))
        .map_err(|errors| ApiError::bad_request(errors.join(" ")))?;

    match stores::create(&state.pool, input).await {
        Ok(store) => Ok(created(store)),
        Err(CreateStoreError::UnknownFranchisee) => Err(ApiError::bad_request(
            "Choose a franchisee that is registered.",
        )),
        Err(CreateStoreError::Database(error)) => Err(error.into()),
    }
}

fn created<T: Serialize>(value: T) -> Response {
    (StatusCode::CREATED, Json(value)).into_response()
}

pub async fn health() -> Json<Health> {
    Json(Health {
        status: "ok",
        database: None,
    })
}

pub async fn ready(State(state): State<AppState>) -> Response {
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await
    {
        Ok(_) => Json(Health {
            status: "ok",
            database: Some("ok"),
        })
        .into_response(),
        Err(error) => {
            tracing::warn!(error = %error, "readiness check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(Health {
                    status: "degraded",
                    database: Some("unavailable"),
                }),
            )
                .into_response()
        }
    }
}

pub async fn stylesheet() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        include_str!("../static/app.css"),
    )
}
