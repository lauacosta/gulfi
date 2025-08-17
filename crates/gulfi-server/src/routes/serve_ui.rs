use axum::{
    http::{StatusCode, Uri},
    response::IntoResponse,
};

use http::{HeaderMap, HeaderValue};
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

        let mut headers = HeaderMap::new();
        headers.insert(
            "Content-Type",
            HeaderValue::from_str(mime_type.as_ref()).unwrap(),
        );

        if file_path.starts_with("assets/") {
            headers.insert(
                "Cache-Control",
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            );
        } else {
            headers.insert("Cache-Control", HeaderValue::from_static("no-cache"));
        }

        (headers, file.contents()).into_response()
    } else {
        warn!("Archivo no encontrado: {:?}", file_path);
        StatusCode::NOT_FOUND.into_response()
    }
}
