use axum::{
    http::{StatusCode, Uri},
    response::IntoResponse,
};

use http::{HeaderMap, HeaderValue};
use include_dir::{Dir, include_dir};
use mime_guess::from_path;
use tracing::{debug, info, instrument};

/// The folder with all the static assets gets generated at compile time by a xtask.
pub static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/static");

#[instrument(level = "debug")]
pub async fn serve_assets(uri: Uri) -> impl IntoResponse {
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
            HeaderValue::from_str(mime_type.as_ref()).expect("It has invalid ASCII characters"),
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
        debug!(path=%file_path, "Asset not found");
        StatusCode::NOT_FOUND.into_response()
    }
}
