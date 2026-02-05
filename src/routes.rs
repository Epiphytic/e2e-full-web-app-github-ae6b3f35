use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rsa::pkcs8::DecodePublicKey;
use rsa::traits::PublicKeyParts;
use rsa::RsaPublicKey;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

use crate::app::AppState;
use crate::auth::AuthUser;
use crate::db::{validate_identifier, ColumnDef};

pub fn api_routes(state: AppState) -> Router {
    Router::new()
        .route("/.well-known/jwks.json", get(jwks_handler))
        // Table management
        .route("/api/tables", get(list_tables).post(create_table))
        .route("/api/tables/{name}", delete(drop_table))
        // Schema management
        .route("/api/tables/{name}/schema", get(get_schema))
        .route("/api/tables/{name}/columns", post(add_column))
        .route(
            "/api/tables/{name}/columns/{col}",
            delete(remove_column),
        )
        // Row management
        .route(
            "/api/tables/{name}/rows",
            get(list_rows).post(insert_row),
        )
        .route(
            "/api/tables/{name}/rows/{id}",
            put(update_row).delete(delete_row),
        )
        .with_state(state)
}

// --- JWKS ---

async fn jwks_handler(State(state): State<AppState>) -> impl IntoResponse {
    let pem_str = match std::str::from_utf8(&state.jwt_validator.public_key_pem) {
        Ok(s) => s,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Invalid public key encoding",
            )
                .into_response()
        }
    };

    let public_key = match RsaPublicKey::from_public_key_pem(pem_str) {
        Ok(k) => k,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to parse public key",
            )
                .into_response()
        }
    };

    let n_bytes = public_key.n().to_bytes_be();
    let e_bytes = public_key.e().to_bytes_be();

    let n_b64 = URL_SAFE_NO_PAD.encode(&n_bytes);
    let e_b64 = URL_SAFE_NO_PAD.encode(&e_bytes);

    let jwks = json!({
        "keys": [{
            "kty": "RSA",
            "alg": "RS256",
            "use": "sig",
            "kid": "jwt-ca-1",
            "n": n_b64,
            "e": e_b64
        }]
    });

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::to_string_pretty(&jwks).unwrap(),
    )
        .into_response()
}

// --- Table Management ---

async fn list_tables(
    _user: AuthUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.db.list_tables() {
        Ok(tables) => (StatusCode::OK, Json(json!({ "tables": tables }))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct CreateTableRequest {
    pub name: String,
    pub columns: Vec<ColumnDef>,
}

async fn create_table(
    _user: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateTableRequest>,
) -> impl IntoResponse {
    if let Err(e) = validate_identifier(&req.name) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response();
    }

    match state.db.create_table(&req.name, &req.columns) {
        Ok(()) => (
            StatusCode::CREATED,
            Json(json!({ "message": format!("Table '{}' created", req.name) })),
        )
            .into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response(),
    }
}

async fn drop_table(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = validate_identifier(&name) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response();
    }

    match state.db.drop_table(&name) {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({ "message": format!("Table '{}' dropped", name) })),
        )
            .into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response(),
    }
}

// --- Schema Management ---

async fn get_schema(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = validate_identifier(&name) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response();
    }

    match state.db.get_table_schema(&name) {
        Ok(schema) => (StatusCode::OK, Json(json!({ "columns": schema }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response(),
    }
}

async fn add_column(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(column): Json<ColumnDef>,
) -> impl IntoResponse {
    if let Err(e) = validate_identifier(&name) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response();
    }

    match state.db.add_column(&name, &column) {
        Ok(()) => (
            StatusCode::CREATED,
            Json(json!({ "message": format!("Column '{}' added to '{}'", column.name, name) })),
        )
            .into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response(),
    }
}

async fn remove_column(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((name, col)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Err(e) = validate_identifier(&name) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response();
    }
    if let Err(e) = validate_identifier(&col) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response();
    }

    match state.db.remove_column(&name, &col) {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({ "message": format!("Column '{}' removed from '{}'", col, name) })),
        )
            .into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response(),
    }
}

// --- Row Management ---

#[derive(Deserialize)]
pub struct PaginationParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

async fn list_rows(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    if let Err(e) = validate_identifier(&name) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response();
    }

    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);

    let total = match state.db.count_rows(&name) {
        Ok(t) => t,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response()
        }
    };

    match state.db.list_rows(&name, limit, offset) {
        Ok((columns, rows)) => (
            StatusCode::OK,
            Json(json!({
                "columns": columns,
                "rows": rows,
                "total": total,
                "limit": limit,
                "offset": offset
            })),
        )
            .into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response(),
    }
}

async fn insert_row(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(values): Json<HashMap<String, String>>,
) -> impl IntoResponse {
    if let Err(e) = validate_identifier(&name) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response();
    }

    match state.db.insert_row(&name, &values) {
        Ok(rowid) => (
            StatusCode::CREATED,
            Json(json!({ "rowid": rowid, "message": "Row inserted" })),
        )
            .into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response(),
    }
}

async fn update_row(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((name, id)): Path<(String, i64)>,
    Json(values): Json<HashMap<String, String>>,
) -> impl IntoResponse {
    if let Err(e) = validate_identifier(&name) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response();
    }

    match state.db.update_row(&name, id, &values) {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({ "message": "Row updated" })),
        )
            .into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response(),
    }
}

async fn delete_row(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((name, id)): Path<(String, i64)>,
) -> impl IntoResponse {
    if let Err(e) = validate_identifier(&name) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response();
    }

    match state.db.delete_row(&name, id) {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({ "message": "Row deleted" })),
        )
            .into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response(),
    }
}
