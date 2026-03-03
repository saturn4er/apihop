use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use apihop_core::import_export::{
    CurlImportResponse, export_apihop, export_curl, export_postman, import_curl, import_openapi,
    import_postman, persist_import,
};
use apihop_core::models::Collection;

use crate::auth::OptionalAuthUser;
use crate::error::AppError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/import/postman", post(import_postman_handler))
        .route("/import/openapi", post(import_openapi_handler))
        .route("/import/curl", post(import_curl_handler))
        .route(
            "/export/collection/{id}/apihop",
            get(export_apihop_handler),
        )
        .route(
            "/export/collection/{id}/postman",
            get(export_postman_handler),
        )
        .route("/export/request/{id}/curl", get(export_curl_handler))
}

#[derive(Deserialize)]
struct ImportBody {
    data: String,
}

// ── Import handlers ──────────────────────────────────────────

async fn import_postman_handler(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Json(body): Json<ImportBody>,
) -> Result<(StatusCode, Json<Collection>), AppError> {
    let result =
        import_postman(&body.data).map_err(|e| AppError::BadRequest(e))?;
    let collection = persist_import(result, state.storage(), auth_user.user_id())
        .await
        .map_err(AppError::from)?;
    Ok((StatusCode::CREATED, Json(collection)))
}

async fn import_openapi_handler(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Json(body): Json<ImportBody>,
) -> Result<(StatusCode, Json<Collection>), AppError> {
    let result =
        import_openapi(&body.data).map_err(|e| AppError::BadRequest(e))?;
    let collection = persist_import(result, state.storage(), auth_user.user_id())
        .await
        .map_err(AppError::from)?;
    Ok((StatusCode::CREATED, Json(collection)))
}

async fn import_curl_handler(
    Json(body): Json<ImportBody>,
) -> Result<Json<CurlImportResponse>, AppError> {
    let req = import_curl(&body.data).map_err(|e| AppError::BadRequest(e))?;
    Ok(Json(req.into()))
}

// ── Export handlers ──────────────────────────────────────────

async fn export_apihop_handler(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (collection, folders, root_requests) =
        apihop_core::load_collection_tree(state.storage(), &id, auth_user.user_id()).await?;
    Ok(Json(export_apihop(&collection, &folders, &root_requests)))
}

async fn export_postman_handler(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (collection, folders, root_requests) =
        apihop_core::load_collection_tree(state.storage(), &id, auth_user.user_id()).await?;
    Ok(Json(export_postman(&collection, &folders, &root_requests)))
}

#[derive(Serialize)]
struct CurlExportResponse {
    curl: String,
}

async fn export_curl_handler(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Path(id): Path<String>,
) -> Result<Json<CurlExportResponse>, AppError> {
    let req = state.storage().get_request(&id, auth_user.user_id()).await?;
    let curl = export_curl(&req);
    Ok(Json(CurlExportResponse { curl }))
}

