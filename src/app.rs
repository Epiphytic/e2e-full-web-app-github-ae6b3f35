use axum::Router;
use std::sync::Arc;

use crate::auth::JwtValidator;
use crate::config::AppConfig;
use crate::db::Database;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub jwt_validator: Arc<JwtValidator>,
    pub config: AppConfig,
}

pub fn create_router(state: AppState) -> Router {
    let templates = crate::views::create_template_engine();

    let view_state = crate::views::ViewState {
        app: state.clone(),
        templates: Arc::new(templates),
    };

    Router::new()
        .merge(crate::views::view_routes(view_state))
        .merge(crate::routes::api_routes(state))
}
