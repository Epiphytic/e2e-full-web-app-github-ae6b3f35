use axum::Router;
use minijinja::Environment;
use std::sync::Arc;

use crate::app::AppState;

#[derive(Clone)]
pub struct ViewState {
    pub app: AppState,
    pub templates: Arc<Environment<'static>>,
}

pub fn create_template_engine() -> Environment<'static> {
    let mut env = Environment::new();
    env.set_loader(minijinja::path_loader("templates"));
    env
}

pub fn view_routes(_state: ViewState) -> Router {
    Router::new()
}
