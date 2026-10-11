use std::collections::HashMap;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use serde_json::{Value, json};

use crate::actions::{ActionError, AppState, execute};
use crate::protocol::tokens_equal;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/status", get(status))
        .route("/v1/size", get(size))
        .route("/v1/position", get(position))
        .route("/v1/on_screen", get(on_screen))
        .route("/v1/pixel", get(pixel))
        .route("/v1/screenshot", get(screenshot))
        .route("/v1/{name}", post(post_action))
        .route("/mcp", post(mcp_post).get(mcp_get).delete(mcp_delete))
        .layer(axum::extract::DefaultBodyLimit::max(32 * 1024 * 1024))
        .with_state(state)
}

async fn post_action(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    if let Some(response) = unauthorized(&headers, &state) {
        return response;
    }
    if name == "abort" {
        state.abort.store(true, std::sync::atomic::Ordering::SeqCst);
        return Json(json!({ "ok": true })).into_response();
    }
    match execute(&state, &name, &body).await {
        Ok(value) => Json(value).into_response(),
        Err(err) => error_response(err),
    }
}

async fn status(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    authed_get(&state, &headers, "status", &json!({})).await
}

async fn size(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    authed_get(&state, &headers, "size", &json!({})).await
}

async fn position(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    authed_get(&state, &headers, "position", &json!({})).await
}

async fn on_screen(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    authed_get(&state, &headers, "on_screen", &query_json(query)).await
}

async fn pixel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    authed_get(&state, &headers, "pixel", &query_json(query)).await
}

async fn screenshot(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    if let Some(response) = unauthorized(&headers, &state) {
        return response;
    }
    match execute(&state, "screenshot", &query_json(query)).await {
        Ok(value) => {
            let Some(png) = value.get("png_base64").and_then(Value::as_str) else {
                return error_response(ActionError::Bad("empty screenshot".into()));
            };
            match base64::engine::general_purpose::STANDARD.decode(png) {
                Ok(bytes) => Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "image/png")
                    .body(Body::from(bytes))
                    .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
                Err(err) => error_response(ActionError::Bad(err.to_string())),
            }
        }
        Err(err) => error_response(err),
    }
}

async fn mcp_post(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    if let Some(response) = unauthorized(&headers, &state) {
        return response;
    }
    match crate::mcp::handle(&state, body).await {
        None => StatusCode::ACCEPTED.into_response(),
        Some(value) => {
            let mut response = Json(value).into_response();
            if let Ok(session) = HeaderValue::from_str(&state.mcp_session) {
                response
                    .headers_mut()
                    .insert(header::HeaderName::from_static("mcp-session-id"), session);
            }
            response
        }
    }
}

async fn mcp_get(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let Some(response) = unauthorized(&headers, &state) {
        return response;
    }
    StatusCode::METHOD_NOT_ALLOWED.into_response()
}

async fn mcp_delete(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let Some(response) = unauthorized(&headers, &state) {
        return response;
    }
    StatusCode::OK.into_response()
}

async fn authed_get(state: &AppState, headers: &HeaderMap, name: &str, args: &Value) -> Response {
    if let Some(response) = unauthorized(headers, state) {
        return response;
    }
    match execute(state, name, args).await {
        Ok(value) => Json(value).into_response(),
        Err(err) => error_response(err),
    }
}

fn unauthorized(headers: &HeaderMap, state: &AppState) -> Option<Response> {
    let presented = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    if presented.is_some_and(|token| tokens_equal(token, &state.token())) {
        None
    } else {
        Some(
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "unauthorized" })),
            )
                .into_response(),
        )
    }
}

fn error_response(err: ActionError) -> Response {
    let status = match err {
        ActionError::NotFound => StatusCode::NOT_FOUND,
        ActionError::Aborted => StatusCode::CONFLICT,
        ActionError::Bad(_) => StatusCode::BAD_REQUEST,
    };
    (status, Json(json!({ "error": err.message() }))).into_response()
}

fn query_json(query: HashMap<String, String>) -> Value {
    let mut map = serde_json::Map::new();
    for (key, value) in query {
        if let Ok(number) = value.parse::<i64>() {
            map.insert(key, json!(number));
        } else if value == "true" || value == "false" {
            map.insert(key, json!(value == "true"));
        } else {
            map.insert(key, json!(value));
        }
    }
    Value::Object(map)
}
