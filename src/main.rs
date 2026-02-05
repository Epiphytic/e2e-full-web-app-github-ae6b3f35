use std::sync::Arc;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use sqlite_editor::app::{AppState, create_router};
use sqlite_editor::auth::JwtValidator;
use sqlite_editor::config::AppConfig;
use sqlite_editor::db::Database;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = AppConfig::from_env();
    tracing::info!("Starting server on {}", config.bind_address());

    let db = Database::new(&config.database_path).expect("Failed to initialize database");
    let jwt_validator = JwtValidator::from_public_key_file(&config.jwt_public_key_path)
        .expect("Failed to load JWT public key");

    let state = AppState {
        db: Arc::new(db),
        jwt_validator: Arc::new(jwt_validator),
        config: config.clone(),
    };

    let app = create_router(state);

    let listener = TcpListener::bind(config.bind_address())
        .await
        .expect("Failed to bind to address");

    tracing::info!("Listening on {}", config.bind_address());
    axum::serve(listener, app).await.expect("Server error");
}
