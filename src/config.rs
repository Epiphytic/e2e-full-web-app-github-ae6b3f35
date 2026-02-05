use std::env;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub port: u16,
    pub database_path: PathBuf,
    pub jwt_public_key_path: PathBuf,
    pub host: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
            database_path: env::var("DATABASE_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("data.db")),
            jwt_public_key_path: env::var("JWT_PUBLIC_KEY_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("certs/jwt-ca.pub")),
            host: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
        }
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
