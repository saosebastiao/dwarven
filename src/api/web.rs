//! Embedded web-UI assets per `coordination-hub.md#R5.2` and
//! `web-ui.md#R2.1`. Three static files compiled into the binary; an SPA
//! fallback serves index.html for any non-API path so client-side
//! deep links work.

use axum::body::Body;
use axum::extract::Request;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;

const INDEX_HTML: &str = include_str!("../../assets/web/index.html");
const APP_JS: &str = include_str!("../../assets/web/app.js");
const APP_CSS: &str = include_str!("../../assets/web/app.css");

pub async fn index() -> Response<Body> {
    static_response(INDEX_HTML, "text/html; charset=utf-8")
}

pub async fn app_js() -> Response<Body> {
    static_response(APP_JS, "application/javascript; charset=utf-8")
}

pub async fn app_css() -> Response<Body> {
    static_response(APP_CSS, "text/css; charset=utf-8")
}

/// Fallback handler. Paths under `/api/` return 404 so we don't paper
/// over typos with HTML; everything else gets the SPA shell so client-side
/// routing handles it.
pub async fn spa_fallback(req: Request) -> Response<Body> {
    let path = req.uri().path();
    if path.starts_with("/api/") {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"error":"not_found","message":"unknown API path"}"#,
            ))
            .expect("build 404");
    }
    static_response(INDEX_HTML, "text/html; charset=utf-8")
}

fn static_response(body: &'static str, content_type: &'static str) -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static(content_type),
        )
        .body(Body::from(body))
        .expect("build static response")
}
