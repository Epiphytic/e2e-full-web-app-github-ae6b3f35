use axum::extract::{FromRef, Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::Router;
use minijinja::{context, Environment};
use serde::Deserialize;
use std::sync::Arc;

use crate::app::AppState;
use crate::auth::{
    build_auth_cookie, build_clear_auth_cookie, AuthUser, AUTH_COOKIE_NAME,
};

#[derive(Clone)]
pub struct ViewState {
    pub app: AppState,
    pub templates: Arc<Environment<'static>>,
}

impl FromRef<ViewState> for AppState {
    fn from_ref(state: &ViewState) -> Self {
        state.app.clone()
    }
}

pub fn create_template_engine() -> Environment<'static> {
    let mut env = Environment::new();
    env.set_loader(minijinja::path_loader("templates"));
    env
}

pub fn view_routes(state: ViewState) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/login", get(login_page).post(login_submit))
        .route("/logout", post(logout_handler))
        .route("/tables", get(tables_page))
        .route("/tables/{name}", get(table_detail_page))
        .with_state(state)
}

async fn index_handler(headers: HeaderMap) -> Response {
    // Check if user has an auth cookie
    let has_cookie = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains(AUTH_COOKIE_NAME))
        .unwrap_or(false);

    if has_cookie {
        Redirect::to("/tables").into_response()
    } else {
        Redirect::to("/login").into_response()
    }
}

async fn login_page(State(state): State<ViewState>) -> impl IntoResponse {
    let tmpl = match state.templates.get_template("login.html") {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template error: {}", e),
            )
                .into_response()
        }
    };

    match tmpl.render(context! { error => "" }) {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Render error: {}", e),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct LoginForm {
    pub token: String,
}

async fn login_submit(
    State(state): State<ViewState>,
    axum::Form(form): axum::Form<LoginForm>,
) -> Response {
    let token = form.token.trim();

    // Validate the token
    match state.app.jwt_validator.validate_token(token) {
        Ok(_claims) => {
            // Set the auth cookie and redirect to tables
            let cookie = build_auth_cookie(token, false);
            let mut response = Redirect::to("/tables").into_response();
            response.headers_mut().insert(
                header::SET_COOKIE,
                cookie.parse().unwrap(),
            );
            response
        }
        Err(e) => {
            // Re-render login page with error
            let tmpl = match state.templates.get_template("login.html") {
                Ok(t) => t,
                Err(te) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Template error: {}", te),
                    )
                        .into_response()
                }
            };

            let error_msg = format!("Invalid token: {}", e);
            match tmpl.render(context! { error => error_msg }) {
                Ok(html) => (StatusCode::UNAUTHORIZED, Html(html)).into_response(),
                Err(re) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Render error: {}", re),
                )
                    .into_response(),
            }
        }
    }
}

async fn logout_handler() -> Response {
    let cookie = build_clear_auth_cookie(false);
    let mut response = Redirect::to("/login").into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookie.parse().unwrap(),
    );
    response
}

async fn tables_page(
    user: AuthUser,
    State(state): State<ViewState>,
) -> Response {
    let tables = match state.app.db.list_tables() {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
                .into_response()
        }
    };

    let tmpl = match state.templates.get_template("tables.html") {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template error: {}", e),
            )
                .into_response()
        }
    };

    match tmpl.render(context! {
        user => context! { username => user.username },
        tables => tables
    }) {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Render error: {}", e),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct TableDetailQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

async fn table_detail_page(
    user: AuthUser,
    State(state): State<ViewState>,
    Path(name): Path<String>,
    Query(params): Query<TableDetailQuery>,
) -> Response {
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);

    let schema = match state.app.db.get_table_schema(&name) {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::NOT_FOUND, format!("Table not found: {}", e)).into_response()
        }
    };

    let total_rows = state.app.db.count_rows(&name).unwrap_or(0);

    let (columns, rows) = match state.app.db.list_rows(&name, limit, offset) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
                .into_response()
        }
    };

    let tmpl = match state.templates.get_template("table_detail.html") {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template error: {}", e),
            )
                .into_response()
        }
    };

    match tmpl.render(context! {
        user => context! { username => user.username },
        table_name => name,
        columns => columns,
        rows => rows,
        schema_columns => schema,
        total_rows => total_rows,
        total => total_rows,
        limit => limit,
        offset => offset
    }) {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Render error: {}", e),
        )
            .into_response(),
    }
}
