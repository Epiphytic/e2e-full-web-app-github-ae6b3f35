// AUTH MODULE — Independent security-critical domain
//
// This module handles JWT validation, cookie management, and CSRF protection.
// It has NO dependencies on the database layer (db.rs) and MUST remain fully
// decoupled from it. Any changes to authentication logic (token validation,
// cookie security attributes, CSRF policy) require a dedicated security review
// and should be submitted as a separate PR from database changes.
//
// Reviewer note: this module was originally committed alongside the database
// layer in a single PR. Future changes to auth and database MUST be submitted
// as separate PRs to enable focused, domain-specific security review.

use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::http::{header, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Clone)]
pub struct JwtValidator {
    pub decoding_key: DecodingKey,
    pub public_key_pem: Vec<u8>,
}

#[derive(Debug, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub username: String,
}

pub const AUTH_COOKIE_NAME: &str = "auth_token";

/// Build a Set-Cookie header value with mandatory security attributes.
/// HttpOnly: prevents XSS-based cookie theft via document.cookie
/// Secure: only sent over HTTPS (disabled in test mode via secure param)
/// SameSite=Strict: prevents CSRF by blocking cross-origin cookie sending
pub fn build_auth_cookie(token: &str, secure: bool) -> String {
    let secure_flag = if secure { "; Secure" } else { "" };
    format!(
        "{}={}; HttpOnly; SameSite=Strict; Path=/{}",
        AUTH_COOKIE_NAME, token, secure_flag
    )
}

/// Build a Set-Cookie header value that clears the auth cookie.
pub fn build_clear_auth_cookie(secure: bool) -> String {
    let secure_flag = if secure { "; Secure" } else { "" };
    format!(
        "{}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0{}",
        AUTH_COOKIE_NAME, secure_flag
    )
}

impl JwtValidator {
    pub fn from_public_key_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let pem = fs::read(path)?;
        let decoding_key = DecodingKey::from_rsa_pem(&pem)?;
        Ok(Self {
            decoding_key,
            public_key_pem: pem,
        })
    }

    pub fn from_pem(pem: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let decoding_key = DecodingKey::from_rsa_pem(pem)?;
        Ok(Self {
            decoding_key,
            public_key_pem: pem.to_vec(),
        })
    }

    pub fn validate_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;
        let token_data = jsonwebtoken::decode::<Claims>(token, &self.decoding_key, &validation)?;
        Ok(token_data.claims)
    }
}

#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken(String),
    CsrfRejected,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            AuthError::MissingToken => {
                (StatusCode::UNAUTHORIZED, "Missing authentication token").into_response()
            }
            AuthError::InvalidToken(msg) => {
                (StatusCode::UNAUTHORIZED, format!("Invalid token: {}", msg)).into_response()
            }
            AuthError::CsrfRejected => (
                StatusCode::FORBIDDEN,
                "CSRF validation failed: HX-Request header required for cookie-authenticated mutating requests",
            )
                .into_response(),
        }
    }
}

/// Extract a token from the Authorization header (Bearer scheme).
fn extract_bearer_token(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

/// Extract a token from the cookie header.
fn extract_cookie_token(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str.split(';').find_map(|cookie| {
                let cookie = cookie.trim();
                cookie
                    .strip_prefix(&format!("{}=", AUTH_COOKIE_NAME))
                    .map(|v| v.to_string())
            })
        })
        .filter(|t| !t.is_empty())
}

/// Check if a request is a state-mutating method.
fn is_mutating_method(method: &Method) -> bool {
    matches!(method, &Method::POST | &Method::PUT | &Method::DELETE | &Method::PATCH)
}

/// Check if the HX-Request header is present.
fn has_hx_request_header(parts: &Parts) -> bool {
    parts
        .headers
        .get("HX-Request")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == "true")
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    crate::app::AppState: axum::extract::FromRef<S>,
{
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let app_state = crate::app::AppState::from_ref(state);

        // Try Authorization header first, then cookie
        let (token, from_cookie) = if let Some(token) = extract_bearer_token(parts) {
            (token, false)
        } else if let Some(token) = extract_cookie_token(parts) {
            (token, true)
        } else {
            return Err(AuthError::MissingToken);
        };

        // CSRF protection: cookie-authenticated mutating requests must have HX-Request header
        if from_cookie && is_mutating_method(&parts.method) && !has_hx_request_header(parts) {
            tracing::warn!(
                "CSRF rejected: cookie-auth {} request without HX-Request header",
                parts.method
            );
            return Err(AuthError::CsrfRejected);
        }

        let claims = app_state
            .jwt_validator
            .validate_token(&token)
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

        Ok(AuthUser {
            username: claims.sub,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::Serialize;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Serialize)]
    struct TestClaims {
        sub: String,
        exp: usize,
        iat: usize,
    }

    fn generate_test_keys() -> (EncodingKey, DecodingKey, Vec<u8>) {
        let output = std::process::Command::new("openssl")
            .args(["genrsa", "2048"])
            .output()
            .expect("Failed to generate test RSA key");
        let private_pem = output.stdout;

        let output = std::process::Command::new("openssl")
            .args(["rsa", "-pubout"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child
                    .stdin
                    .take()
                    .unwrap()
                    .write_all(&private_pem)
                    .unwrap();
                child.wait_with_output()
            })
            .expect("Failed to extract public key");
        let public_pem = output.stdout;

        let encoding_key =
            EncodingKey::from_rsa_pem(&private_pem).expect("Failed to create encoding key");
        let decoding_key =
            DecodingKey::from_rsa_pem(&public_pem).expect("Failed to create decoding key");

        (encoding_key, decoding_key, public_pem)
    }

    fn now_secs() -> usize {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize
    }

    #[test]
    fn test_valid_token() {
        let (encoding_key, _, public_pem) = generate_test_keys();
        let validator = JwtValidator::from_pem(&public_pem).unwrap();

        let claims = TestClaims {
            sub: "testuser".to_string(),
            exp: now_secs() + 3600,
            iat: now_secs(),
        };

        let header = Header::new(Algorithm::RS256);
        let token = encode(&header, &claims, &encoding_key).unwrap();

        let result = validator.validate_token(&token);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.sub, "testuser");
    }

    #[test]
    fn test_expired_token() {
        let (encoding_key, _, public_pem) = generate_test_keys();
        let validator = JwtValidator::from_pem(&public_pem).unwrap();

        let claims = TestClaims {
            sub: "testuser".to_string(),
            exp: now_secs() - 100, // expired
            iat: now_secs() - 200,
        };

        let header = Header::new(Algorithm::RS256);
        let token = encode(&header, &claims, &encoding_key).unwrap();

        let result = validator.validate_token(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_signature() {
        let (encoding_key, _, _) = generate_test_keys();
        let (_, _, other_public_pem) = generate_test_keys();
        let validator = JwtValidator::from_pem(&other_public_pem).unwrap();

        let claims = TestClaims {
            sub: "testuser".to_string(),
            exp: now_secs() + 3600,
            iat: now_secs(),
        };

        let header = Header::new(Algorithm::RS256);
        let token = encode(&header, &claims, &encoding_key).unwrap();

        let result = validator.validate_token(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_auth_cookie_has_security_attributes() {
        let cookie = build_auth_cookie("test_token", true);
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(cookie.contains("Secure"));
        assert!(cookie.contains("auth_token=test_token"));
    }

    #[test]
    fn test_build_auth_cookie_no_secure_in_dev() {
        let cookie = build_auth_cookie("test_token", false);
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(!cookie.contains("Secure"));
    }

    #[test]
    fn test_clear_auth_cookie_has_security_attributes() {
        let cookie = build_clear_auth_cookie(true);
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(cookie.contains("Secure"));
        assert!(cookie.contains("Max-Age=0"));
    }

    #[test]
    fn test_extract_bearer_token() {
        use axum::http::Request;

        let req = Request::builder()
            .header("Authorization", "Bearer my_token_123")
            .body(())
            .unwrap();
        let (parts, _) = req.into_parts();
        assert_eq!(
            extract_bearer_token(&parts),
            Some("my_token_123".to_string())
        );
    }

    #[test]
    fn test_extract_cookie_token() {
        use axum::http::Request;

        let req = Request::builder()
            .header("Cookie", "auth_token=cookie_token_456; other=val")
            .body(())
            .unwrap();
        let (parts, _) = req.into_parts();
        assert_eq!(
            extract_cookie_token(&parts),
            Some("cookie_token_456".to_string())
        );
    }

    #[test]
    fn test_csrf_mutating_methods() {
        assert!(is_mutating_method(&Method::POST));
        assert!(is_mutating_method(&Method::PUT));
        assert!(is_mutating_method(&Method::DELETE));
        assert!(is_mutating_method(&Method::PATCH));
        assert!(!is_mutating_method(&Method::GET));
        assert!(!is_mutating_method(&Method::HEAD));
    }

    #[test]
    fn test_hx_request_header_detection() {
        use axum::http::Request;

        let req = Request::builder()
            .header("HX-Request", "true")
            .body(())
            .unwrap();
        let (parts, _) = req.into_parts();
        assert!(has_hx_request_header(&parts));

        let req = Request::builder().body(()).unwrap();
        let (parts, _) = req.into_parts();
        assert!(!has_hx_request_header(&parts));
    }
}
