//! Localhost catalog for the Vite page opened in a normal browser.
//! The Tauri webview already has `invoke`. A browser tab does not, so debug
//! builds also serve the model list on 127.0.0.1. The API key is never returned.

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::body::Body;
use axum::extract::{Query, Request, State};
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde::Deserialize;
use serde_json::json;

use crate::actions::AppState;
use crate::{settings_json, write_llm};

pub const PORT: u16 = 1429;

pub fn spawn(state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        let router = Router::new()
            .route("/llm/providers", get(providers).options(preflight))
            .route("/llm/catalog", get(catalog).options(preflight))
            .route("/llm/settings", get(settings).post(save).options(preflight))
            .layer(axum::middleware::from_fn(cors))
            .with_state(state);
        let listener = match tokio::net::TcpListener::bind(("127.0.0.1", PORT)).await {
            Ok(listener) => listener,
            Err(err) => {
                eprintln!("browser model api: {err}");
                return;
            }
        };
        let _ = axum::serve(listener, router).await;
    });
}

async fn providers() -> impl IntoResponse {
    Json(crate::llm::providers())
}

async fn catalog(Query(query): Query<CatalogQuery>) -> Response {
    match crate::llm::catalog_models(&query.provider) {
        Ok(models) => Json(models).into_response(),
        Err(err) => error(err),
    }
}

async fn settings(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(settings_json(&state))
}

async fn save(State(state): State<Arc<AppState>>, Json(body): Json<SaveBody>) -> Response {
    match write_llm(
        &state,
        body.provider,
        body.base_url,
        body.model,
        body.api_key,
    ) {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(err) => error(err),
    }
}

async fn preflight() -> StatusCode {
    StatusCode::NO_CONTENT
}

fn error(message: String) -> Response {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": message }))).into_response()
}

#[derive(Deserialize)]
struct CatalogQuery {
    provider: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveBody {
    provider: String,
    base_url: String,
    model: String,
    api_key: Option<String>,
}

async fn cors(request: Request<Body>, next: Next) -> Response {
    let origin = allowed_origin(request.headers().get(header::ORIGIN));
    if request.method() == Method::OPTIONS {
        let mut response = StatusCode::NO_CONTENT.into_response();
        apply_cors(response.headers_mut(), origin.as_deref());
        return response;
    }
    let mut response = next.run(request).await;
    apply_cors(response.headers_mut(), origin.as_deref());
    response
}

fn allowed_origin(value: Option<&HeaderValue>) -> Option<String> {
    let origin = value?.to_str().ok()?;
    if origin == "http://localhost:1420" || origin == "http://127.0.0.1:1420" {
        Some(origin.to_string())
    } else {
        None
    }
}

fn apply_cors(headers: &mut axum::http::HeaderMap, origin: Option<&str>) {
    let Some(origin) = origin else {
        return;
    };
    if let Ok(value) = HeaderValue::from_str(origin) {
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, value);
    }
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, OPTIONS"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("content-type"),
    );
    headers.insert(header::VARY, HeaderValue::from_static("origin"));
}
