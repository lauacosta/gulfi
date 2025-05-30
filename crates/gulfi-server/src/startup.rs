use axum::error_handling::HandleErrorLayer;
use axum::{BoxError, Extension};
use color_eyre::owo_colors::OwoColorize;
use eyre::{Result, eyre};
use gulfi_common::Document;
use gulfi_sqlite::pooling::AsyncConnectionPool;
use gulfi_sqlite::spawn_vec_connection;
use http::{Method, StatusCode};
use moka::future::Cache;
use rusqlite::{Connection, params};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{fmt, io};
use tokio::sync::mpsc::{self, UnboundedSender};
use tower::buffer::BufferLayer;
// use tower::limit::RateLimitLayer;
use tower_http::LatencyUnit;
use tower_http::cors::Any;
use tower_http::{compression::CompressionLayer, cors::CorsLayer};

use axum::{Router, body::Body, http::Request, routing::get, serve::Serve};
use tokio::{net::TcpListener, signal};
use tower::ServiceBuilder;
use tower_http::trace::{OnResponse, TraceLayer};
use tower_request_id::{RequestId, RequestIdLayer};
use tracing::{Level, debug_span, error, info};

use crate::ApplicationSettings;
use crate::routes::{
    add_favoritos, auth, delete_favoritos, delete_historial, documents, favoritos, health_check,
    historial_detailed, historial_summary, search, serve_ui,
};
use crate::search::SearchStrategy;

#[derive(Debug, Clone)]
pub struct ServerState {
    pub documents: Vec<Document>,
    pub writer: UnboundedSender<WriteJob>,
    pub pool: AsyncConnectionPool,
    pub embeddings_cache: Cache<String, Arc<Vec<f32>>>,
}

#[derive(Debug)]
pub struct Application {
    pub port: u16,
    pub host: IpAddr,
    pub server: Serve<Router, Router>,
}

impl Application {
    /// # Errors
    /// Fails if it's not capable of acquiring a port for`tokio::net::TcpListener`.
    ///
    /// # Panics
    /// It panics if it's not able to get a port for the given address.
    ///
    pub async fn build(
        configuration: &ApplicationSettings,
        documents: Vec<Document>,
    ) -> Result<Self> {
        let address = format!("{}:{}", configuration.host, configuration.port);

        let listener = match TcpListener::bind(&address).await {
            Ok(listener) => listener,
            Err(err) => {
                error!("{err}. Trying with another port...");
                match TcpListener::bind(format!("{}:0", configuration.host)).await {
                    Ok(listener) => listener,
                    Err(err) => {
                        error!("There aren't available ports, closing application...");
                        return Err(err.into());
                    }
                }
            }
        };

        let port = listener
            .local_addr()
            .expect("It should be able to find the locall address")
            .port();

        let host = configuration.host;

        let db_path = std::env::var("DATABASE_URL").map_err(|err| {
            eyre!(
                "Environment variable `DATABASE_URL` is not set. Err: {}",
                err
            )
        })?;

        let pool = AsyncConnectionPool::new(10, || spawn_vec_connection(&db_path))?;

        let writer = spawn_writer_task(&db_path)?;

        let state = ServerState {
            documents,
            writer,
            pool,
            embeddings_cache: Cache::builder()
                // TTL
                .time_to_live(Duration::from_secs(5 * 60))
                // TTI
                .time_to_idle(Duration::from_secs(60))
                .build(),
        };

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
    /// Fails if there is an inconvenient while programming the async task.
    ///
    /// # Panics
    /// Panics if it is unable to install the handler.
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
                        info!("ctrl+c detectado.");
                    },
                    () = terminate => {
                        info!("ctrl+c detectado.");
                    },
                }
            })
            .await
    }
}

pub fn build_server(listener: TcpListener, state: ServerState) -> Result<Serve<Router, Router>> {
    let historial_routes = Router::new()
        .route(
            "/:doc/historial",
            get(historial_summary).delete(delete_historial),
        )
        .route("/:doc/historial-full", get(historial_detailed));

    let search_routes = Router::new().route("/search", get(search)).layer(
        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(|err: BoxError| async move {
                (
                    StatusCode::TOO_MANY_REQUESTS,
                    format!("Unhandled error {err}"),
                )
            }))
            .layer(BufferLayer::new(1024)), // .layer(RateLimitLayer::new(1000, Duration::from_secs(1))),
    );

    let frontend_routes = Router::new()
        .route("/assets/*path", get(serve_ui))
        .fallback(serve_ui);

    let api_routes = Router::new()
        .nest("/api", search_routes)
        .nest("/api", historial_routes)
        .route("/api/auth", get(auth))
        .route("/api/health_check", get(health_check))
        .route(
            "/api/:doc/favoritos",
            get(favoritos).post(add_favoritos).delete(delete_favoritos),
        )
        .route("/api/documents", get(documents));

    let mut server = api_routes.merge(frontend_routes).with_state(state);

    if cfg!(debug_assertions) {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::POST, Method::DELETE])
            .allow_headers(Any);

        server = server.layer(cors);
    }

    let server = server
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
                                .map_or_else(|| "unknown".into(), ToString::to_string);

                            debug_span!(
                                "request",
                                id = %request_id,
                                method = %request.method().blue().bold(),
                                uri = %request.uri(),
                            )
                        })
                        .on_response(
                            ColoredOnResponse::new()
                                .include_headers(true)
                                .latency_unit(LatencyUnit::Millis)
                                .level(Level::INFO),
                        ),
                )
                .layer(RequestIdLayer),
        )
        .layer(CompressionLayer::new());

    Ok(axum::serve(listener, server))
}

pub async fn run_server(
    configuration: ApplicationSettings,
    start: Instant,
    documents: Vec<Document>,
) -> Result<()> {
    match Application::build(&configuration, documents).await {
        Ok(app) => {
            let url = format!("http://{}:{}", app.host(), app.port());

            eprintln!(
                "\n\n  {} {} ready in {} ms\n",
                configuration.name.to_uppercase().bold().bright_green(),
                format!("v{}", configuration.version).green(),
                start.elapsed().as_millis().bold().bright_white(),
            );

            eprintln!(
                "  {}  {}:  {}\n\n",
                "➜".bold().bright_green(),
                "Local".bold().bright_white(),
                url.bright_cyan().underline()
            );

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

#[derive(Clone)]
struct ColoredOnResponse {
    level: Level,
    latency_unit: LatencyUnit,
    include_headers: bool,
}

impl ColoredOnResponse {
    fn new() -> Self {
        Self {
            level: Level::INFO,
            latency_unit: LatencyUnit::Millis,
            include_headers: false,
        }
    }

    fn level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    pub fn latency_unit(mut self, latency_unit: LatencyUnit) -> Self {
        self.latency_unit = latency_unit;
        self
    }

    fn include_headers(mut self, include: bool) -> Self {
        self.include_headers = include;
        self
    }
}

struct Latency {
    unit: LatencyUnit,
    duration: Duration,
}

impl fmt::Display for Latency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.unit {
            LatencyUnit::Seconds => write!(f, "{} s", self.duration.as_secs_f64()),
            LatencyUnit::Millis => write!(f, "{} ms", self.duration.as_millis()),
            LatencyUnit::Micros => write!(f, "{} μs", self.duration.as_micros()),
            LatencyUnit::Nanos => write!(f, "{} ns", self.duration.as_nanos()),
            _ => write!(f, "unknown unitdesconocida."),
        }
    }
}

macro_rules! event_dynamic_lvl {
    ( $(target: $target:expr,)? $(parent: $parent:expr,)? $lvl:expr, $($tt:tt)* ) => {
        match $lvl {
            tracing::Level::ERROR => {
                tracing::event!(
                    $(target: $target,)?
                    $(parent: $parent,)?
                    tracing::Level::ERROR,
                    $($tt)*
                );
            }
            tracing::Level::WARN => {
                tracing::event!(
                    $(target: $target,)?
                    $(parent: $parent,)?
                    tracing::Level::WARN,
                    $($tt)*
                );
            }
            tracing::Level::INFO => {
                tracing::event!(
                    $(target: $target,)?
                    $(parent: $parent,)?
                    tracing::Level::INFO,
                    $($tt)*
                );
            }
            tracing::Level::DEBUG => {
                tracing::event!(
                    $(target: $target,)?
                    $(parent: $parent,)?
                    tracing::Level::DEBUG,
                    $($tt)*
                );
            }
            tracing::Level::TRACE => {
                tracing::event!(
                    $(target: $target,)?
                    $(parent: $parent,)?
                    tracing::Level::TRACE,
                    $($tt)*
                );
            }
        }
    };
}

impl<B> OnResponse<B> for ColoredOnResponse {
    fn on_response(self, response: &http::Response<B>, latency: Duration, _: &tracing::Span) {
        let latency = Latency {
            unit: self.latency_unit,
            duration: latency,
        };

        let response_headers = self
            .include_headers
            .then(|| tracing::field::debug(response.headers()));

        let status = response.status();
        let colored_status = if status.is_success() {
            format!("{}", status.bright_green().bold())
        } else if status.is_client_error() {
            format!("{}", status.bright_yellow().bold())
        } else if status.is_server_error() {
            format!("{}", status.bright_red().bold())
        } else {
            format!("{}", status.bright_cyan().bold())
        };

        event_dynamic_lvl!(
            self.level,
            latency = %format!("{}", latency.bright_blue().bold()),
            status = %colored_status,
            response_headers,
            "request procesado"
        );
    }
}

#[derive(Debug)]
pub enum WriteJob {
    Historial {
        query: String,
        doc: String,
        strategy: SearchStrategy,
        peso_fts: f32,
        peso_semantic: f32,
        k_neighbors: u64,
    },
    Cache {
        query: String,
        result_json: String,
        expires_at: i64,
    },
}

fn spawn_writer_task(db_path: &str) -> eyre::Result<mpsc::UnboundedSender<WriteJob>> {
    let conn = Connection::open(db_path)?;
    let (tx, mut rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        while let Some(job) = rx.recv().await {
            let res = match job {
                WriteJob::Historial {
                    query,
                    doc,
                    strategy,
                    peso_fts,
                    peso_semantic,
                    k_neighbors,
                } => {
                    conn.execute(
                        "insert or replace into historial(query, strategy, doc, peso_fts, peso_semantic, neighbors) values (?,?,?,?,?,?)",
                        params![query, strategy, doc, peso_fts, peso_semantic, k_neighbors],
                    )
                }
                WriteJob::Cache {
                    query: _,
                    result_json: _,
                    expires_at: _,
                } => todo!(),
            };

            if let Err(e) = res {
                eprintln!("[writer task] Write failed: {e:?}");
            }
        }
    });

    Ok(tx)
}
