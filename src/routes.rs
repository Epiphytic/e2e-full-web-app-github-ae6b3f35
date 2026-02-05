use axum::Router;

use crate::app::AppState;

pub fn api_routes(_state: AppState) -> Router {
    Router::new()
}
