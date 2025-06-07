use axum::{
    BoxError, Extension, Router, body::Body, error_handling::HandleErrorLayer, http::Request,
    routing::get, serve::Serve,
};

use color_eyre::owo_colors::OwoColorize;
use eyre::Result;
use gulfi_common::Document;
use gulfi_openai::OpenAIClient;
use gulfi_sqlite::{pooling::AsyncConnectionPool, spawn_vec_connection};
use http::{Method, StatusCode};
use moka::future::Cache;
use secrecy::ExposeSecret;
use std::{fmt, io};
use std::{
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::mpsc::UnboundedSender;
use tower::buffer::BufferLayer;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::{OnResponse, TraceLayer},
};

use tokio::{net::TcpListener, signal};
use tower::ServiceBuilder;
use tower_request_id::{RequestId, RequestIdLayer};
use tracing::{Instrument, Level, debug_span, error, info, info_span};

use crate::bg_tasks::{WriteJob, spawn_writer_task};
use crate::configuration::Settings;
use crate::routes::{
    add_favoritos, auth, delete_favoritos, delete_historial, documents, favoritos, health_check,
    historial_detailed, historial_summary, search, serve_ui,
};

#[derive(Debug, Clone)]
pub struct ServerState {
    pub documents: Vec<Document>,
    pub writer: UnboundedSender<WriteJob>,
    pub embeddings_provider: OpenAIClient,
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
    pub async fn build(configuration: &Settings, documents: Vec<Document>) -> Result<Self> {
        let pool_size = configuration.db_settings.pool_size;
        let db_path = configuration.db_settings.db_path.clone();
        let pool = AsyncConnectionPool::new(pool_size, || spawn_vec_connection(&db_path))?;

        // TODO: A more generic Client for embeddings
        let embeddings_provider = OpenAIClient::new(
            configuration
                .embedding_provider
                .auth_token
                .clone()
                .expose_secret()
                .to_string(),
            configuration.embedding_provider.endpoint_url.clone(),
        );

        let address = format!(
            "{}:{}",
            configuration.app_settings.host, configuration.app_settings.port
        );

        let host = configuration.app_settings.host;
        let listener = match TcpListener::bind(&address).await {
            Ok(listener) => listener,
            Err(err) => {
                error!("{err}. Trying with another port...");
                match TcpListener::bind(format!("{host}:0")).await {
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

        let writer = spawn_writer_task(&db_path)?;

        let state = ServerState {
            documents,
            writer,
            embeddings_provider,
            pool,
            embeddings_cache: Cache::builder()
                // TTL
                .time_to_live(Duration::from_secs(5 * 60))
                // TTI
                .time_to_idle(Duration::from_secs(60))
                .build(),
        };

        Ok(Self {
            port,
            host,
            server: build_server(listener, state)?,
        })
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
            .with_graceful_shutdown(
                async move {
                    let ctrl_c = async {
                        signal::ctrl_c()
                            .await
                            .expect("failed to install handler for ctrl+c")
                    };

                    #[cfg(unix)]
                    let terminate = async {
                        signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                            .expect("failed to install handler for ctrl+c")
                            .recv()
                            .await;
                    };

                    #[cfg(not(unix))]
                    let terminate = std::future::pending::<()>();

                    tokio::select! {
                        () = ctrl_c => {
                            info!("Received SIGINT");
                            info!("Exiting immediately");
                        },
                        () = terminate => {
                            info!("Received SIGINT");
                            info!("Exiting immediately");
                        },
                    }
                }
                .instrument(info_span!("graceful-shutdown")),
            )
            .await
    }
}

pub fn build_server(listener: TcpListener, state: ServerState) -> Result<Serve<Router, Router>> {
    let historial_routes = Router::new()
        .route(
            "/:doc/history",
            get(historial_summary).delete(delete_historial),
        )
        .route("/:doc/history-full", get(historial_detailed));

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
            "/api/:doc/favorites",
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

                            let user_agent = request
                                .headers()
                                .get("user-agent")
                                .and_then(|h| h.to_str().ok())
                                .unwrap_or("unkown");

                            debug_span!(
                                "request",
                                id = %request_id,
                                method = %request.method().blue().bold(),
                                uri = %request.uri(),
                                user_agent= %user_agent
                            )
                        })
                        .on_response(
                            ColoredOnResponse::new()
                                .include_headers(true)
                                .level(Level::INFO),
                        ),
                )
                .layer(RequestIdLayer),
        )
        .layer(CompressionLayer::new());

    Ok(axum::serve(listener, server))
}

pub async fn run_server(
    configuration: Settings,
    start: Instant,
    documents: Vec<Document>,
    open: bool,
) -> Result<()> {
    match Application::build(&configuration, documents).await {
        Ok(app) => {
            let url = format!("http://{}:{}", app.host(), app.port());
            let name = configuration.app_settings.name;
            let version = env!("CARGO_PKG_VERSION");

            eprintln!(
                "\n\n  {} {} ready in {} ms\n",
                name.to_uppercase().bold().bright_green(),
                format!("v{version}").green(),
                start.elapsed().as_millis().bold().bright_white(),
            );

            eprintln!(
                "  {}  {}:  {}\n\n",
                "➜".bold().bright_green(),
                "Local".bold().bright_white(),
                url.bright_cyan().underline()
            );

            if open && webbrowser::open_browser(webbrowser::Browser::Default, &url).is_ok() {
                info!("App will open on default browser if enabled");
            }

            if let Err(e) = app.run_until_stopped().await {
                error!("Error executing HTTP server: {:?}", e);
                return Err(e.into());
            }
        }
        Err(e) => {
            error!("Fail at starting server: {:?}", e);
            return Err(e);
        }
    }
    Ok(())
}

#[derive(Clone)]
struct ColoredOnResponse {
    level: Level,
    include_headers: bool,
}

impl ColoredOnResponse {
    fn new() -> Self {
        Self {
            level: Level::INFO,
            include_headers: false,
        }
    }

    fn level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    fn include_headers(mut self, include: bool) -> Self {
        self.include_headers = include;
        self
    }
}

struct Latency {
    duration: Duration,
}

impl Latency {
    fn new(duration: Duration) -> Self {
        Self { duration }
    }

    fn best_unit_and_value(&self) -> (f64, &'static str) {
        let nanos = self.duration.as_nanos();

        if nanos >= 1_000_000_000 {
            (self.duration.as_secs_f64(), "s")
        } else if nanos >= 1_000_000 {
            (self.duration.as_millis() as f64, "ms")
        } else if nanos >= 1_000 {
            (self.duration.as_micros() as f64, "μs")
        } else {
            (nanos as f64, "ns")
        }
    }
}

impl fmt::Display for Latency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (value, unit) = self.best_unit_and_value();

        write!(f, "{value:.3} {unit}")
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
        let latency = Latency::new(latency);

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
