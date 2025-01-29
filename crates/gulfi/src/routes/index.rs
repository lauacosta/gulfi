use gulfi_ui::Index;

#[tracing::instrument(name = "Sirviendo la página inicial")]
#[axum::debug_handler]
// pub async fn index(State(app): State<AppState>) -> eyre::Result<Index, HttpError> {
//     let db = Connection::open(app.db_path)
//         .expect("Deberia ser un path valido a una base de datos SQLite");
//     let historial = get_historial(&db)?;
//
//     Ok(Index {})
//
// }
pub async fn index() -> Index {
    // let db = Connection::open(app.db_path)
    //     .expect("Deberia ser un path valido a una base de datos SQLite");
    // let historial = get_historial(&db)?;

    Index {}
}
