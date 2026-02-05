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
