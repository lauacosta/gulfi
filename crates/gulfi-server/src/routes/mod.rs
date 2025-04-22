mod serve_ui;
pub use serve_ui::*;
mod favoritos;
pub use favoritos::*;
pub use health_check::*;
mod health_check;
mod historial;
pub use historial::*;
pub(crate) mod search;
pub use search::*;

mod documentos;
pub use documentos::*;
