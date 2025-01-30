use axum::Extension;
use eyre::Result;
use gulfi_cli::Cache;
use gulfi_configuration::ApplicationSettings;
use gulfi_sqlite::init_sqlite;
use std::io;
use std::net::IpAddr;
use std::time::Duration;
use tower_http::compression::CompressionLayer;

use axum::{Router, body::Body, http::Request, routing::get, serve::Serve};
use tokio::{net::TcpListener, signal};
use tower::ServiceBuilder;
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tower_request_id::{RequestId, RequestIdLayer};
use tracing::{Level, error, error_span, info, instrument};

use crate::routes::{
    add_favoritos, delete_favoritos, delete_historial, favoritos, handle_assets, health_check,
    historial, index, search,
};

#[derive(Debug, Clone)]
pub struct AppState {
    pub db_path: String,
    pub cache: Cache,
}

#[derive(Debug)]
pub struct Application {
    pub port: u16,
    pub host: IpAddr,
    pub server: Serve<Router, Router>,
}

impl Application {
    /// # Errors
    /// Fallará si no logra obtener la direccion local del `tokio::net::TcpListener`.
    ///
    /// # Panics
    /// Entrará en panicos si no es capaz de:
    /// 1. Vincular un `tokio::net::TcpListener` a la dirección dada.
    /// 2. Falla en conectarse con el servidor de `MeiliSearch`.
    #[tracing::instrument(name = "Construyendo la aplicación.", skip(configuration))]
    pub async fn build(configuration: &ApplicationSettings) -> Result<Self> {
        let address = format!("{}:{}", configuration.host, configuration.port);

        let listener = match TcpListener::bind(&address).await {
            Ok(listener) => listener,
            Err(err) => {
                error!("{err}. Tratando con otro puerto...");
                match TcpListener::bind(format!("{}:0", configuration.host)).await {
                    Ok(listener) => listener,
                    Err(err) => {
                        error!("No hay puertos disponibles, finalizando la aplicación...");
                        return Err(err.into());
                    }
                }
            }
        };

        let port = listener
            .local_addr()
            .expect("Fallo al encontrar la local address")
            .port();

        let host = configuration.host;

        let db_path = init_sqlite()?;
        let cache = configuration.cache.clone();

        let state = AppState { db_path, cache };

        let server = build_server(listener, state)?;

        Ok(Self { port, host, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn host(&self) -> String {
        self.host.to_string()
    }

    /// # Errors
    ///
    /// Devolverá error si ocurre algun inconveniente con tokio para programar la tarea asíncrona.
    /// # Panics
    ///
    /// Entrará en pánico si no es capaz de instalar el handler requerido.
    #[tracing::instrument(skip(self))]
    pub async fn run_until_stopped(self) -> io::Result<()> {
        self.server
            // https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs
            .with_graceful_shutdown(async move {
                let ctrl_c = async {
                    signal::ctrl_c()
                        .await
                        .expect("Fallo en instalar el handler para Ctrl+C");
                };
                #[cfg(unix)]
                let terminate = async {
                    signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                        .expect("Fallo en instalar el handler para las señales")
                        .recv()
                        .await;
                };

                #[cfg(not(unix))]
                let terminate = std::future::pending::<()>();

                tokio::select! {
                    () = ctrl_c => {
                        info!("ctrl+c detectado.")
                    },
                    () = terminate => {
                        info!("ctrl+c detectado.")
                    },
                }
            })
            .await
    }
}

pub fn build_server(listener: TcpListener, state: AppState) -> Result<Serve<Router, Router>> {
    let server = Router::new()
        .route("/", get(index))
        .route("/health", get(health_check))
        .route(
            "/favoritos",
            get(favoritos).post(add_favoritos).delete(delete_favoritos),
        )
        .route("/search", get(search))
        .route("/historial", get(historial).delete(delete_historial))
        .route("/_assets/*path", get(handle_assets))
        .with_state(state)
        .layer(Extension(
            reqwest::ClientBuilder::new()
                .timeout(Duration::from_secs(5))
                .build()?,
        ))
        .layer(
            ServiceBuilder::new()
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(|request: &Request<Body>| {
                            let request_id = request
                                .extensions()
                                .get::<RequestId>()
                                .map_or_else(|| "desconocido".into(), ToString::to_string);

                            error_span!(
                                "request",
                                id = %request_id,
                                method = %request.method(),
                                uri = %request.uri()
                            )
                        })
                        .on_response(
                            DefaultOnResponse::new()
                                .include_headers(true)
                                .level(Level::INFO),
                        ),
                )
                .layer(RequestIdLayer),
        )
        .layer(CompressionLayer::new());

    Ok(axum::serve(listener, server))
}

#[instrument(skip(configuration))]
pub async fn run_server(configuration: ApplicationSettings) -> Result<()> {
    match Application::build(&configuration).await {
        Ok(app) => {
            let url = format!("http://{}:{}", app.host(), app.port());
            info!("La aplicación está disponible en {url}");

            if configuration.open
                && webbrowser::open_browser(webbrowser::Browser::Default, &url).is_ok()
            {
                info!("Se abrirá la aplicación en el navegador predeterminado.");
            }

            if let Err(e) = app.run_until_stopped().await {
                error!("Error ejecutando el servidor HTTP: {:?}", e);
                return Err(e.into());
            }
        }
        Err(e) => {
            error!("Fallo al iniciar el servidor: {:?}", e);
            return Err(e);
        }
    }
    Ok(())
}
