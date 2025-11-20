use axum::{
    http::{header, StatusCode, Uri},
    response::IntoResponse,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "ui/dist"]
struct Asset;

pub struct UIHandler;

impl UIHandler {
    pub async fn serve(uri: Uri) -> impl IntoResponse {
        let path = uri.path().trim_start_matches('/');
        
        let path = if path.is_empty() || path == "index.html" {
            "index.html"
        } else {
            path
        };

        match Asset::get(path) {
            Some(content) => {
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
            }
            None => {
                // SPA Fallback: serve index.html for unknown paths (handled by client-side router)
                if let Some(content) = Asset::get("index.html") {
                    let mime = mime_guess::from_path("index.html").first_or_octet_stream();
                    ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
                } else {
                    (StatusCode::NOT_FOUND, "404 Not Found").into_response()
                }
            }
        }
    }
}
