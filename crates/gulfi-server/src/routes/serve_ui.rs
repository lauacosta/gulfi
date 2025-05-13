use axum::{
    http::{StatusCode, Uri},
    response::IntoResponse,
};

use mime_guess::from_path;
use tracing::{info, instrument, warn};

use crate::ASSETS;

#[instrument]
pub async fn serve_ui(uri: Uri) -> impl IntoResponse {
    let path = uri.path();

    let file_path = if path.starts_with("/assets/") {
        &format!("assets{}", path.trim_start_matches("/assets"))
    } else {
        "index.html"
    };

    let file = ASSETS.get_file(file_path);

    match file {
        Some(file) => {
            let mime_type = from_path(file_path).first_or_octet_stream();
            info!("{:?}, MIME: {:?}", file_path, mime_type);
            ([("Content-Type", mime_type.as_ref())], file.contents()).into_response()
        }
        None => {
            warn!("Archivo no encontrado: {:?}", file_path);
            StatusCode::NOT_FOUND.into_response()
        }
    }
}
