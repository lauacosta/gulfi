use axum::{
    http::{StatusCode, Uri},
    response::IntoResponse,
};

use http::header;
use mime_guess::from_path;
use tracing::{info, instrument, warn};

use crate::ASSETS;

#[instrument(level = "info")]
pub async fn serve_ui(uri: Uri) -> impl IntoResponse {
    let path = uri.path();

    let file_path = if path.starts_with("/assets/") {
        &format!("assets{}", path.trim_start_matches("/assets"))
    } else {
        "index.html"
    };

    if let Some(file) = ASSETS.get_file(file_path) {
        let mime_type = from_path(file_path).first_or_octet_stream();
        info!("{:?}, MIME: {:?}", file_path, mime_type);
        let cache_control = if file_path == "index.html" {
            "no-cache"
        } else {
            "public, max-age=31536000, immutable"
        };

        (
            [
                (http::header::CONTENT_TYPE, mime_type.as_ref()),
                (header::CACHE_CONTROL, cache_control),
            ],
            file.contents(),
        )
            .into_response()
    } else {
        warn!("File not found: {:?}", file_path);
        (StatusCode::NOT_FOUND, "Not Found").into_response()
    }
}
