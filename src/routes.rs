use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rsa::pkcs8::DecodePublicKey;
use rsa::traits::PublicKeyParts;
use rsa::RsaPublicKey;
use serde_json::json;

use crate::app::AppState;

pub fn api_routes(state: AppState) -> Router {
    Router::new()
        .route("/.well-known/jwks.json", get(jwks_handler))
        .with_state(state)
}

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
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to parse public key")
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
