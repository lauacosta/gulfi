use axum::{
    BoxError, Extension, Router, body::Body, error_handling::HandleErrorLayer, http::Request,
    routing::get, serve::Serve,
};
use gulfi_ingest::Document;
use opentelemetry::trace::TraceContextExt;
use reqwest::Client;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use color_eyre::owo_colors::OwoColorize;
use eyre::Result;
use gulfi_ingest::{pool::AsyncConnectionPool, spawn_vec_connection};
use gulfi_openai::OpenAIClient;
use http::{Method, StatusCode};
use moka::future::Cache;
use secrecy::ExposeSecret;
use std::io;
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
    trace::TraceLayer,
};

use tokio::{net::TcpListener, signal};
use tower::ServiceBuilder;
use tower_request_id::{RequestId, RequestIdLayer};
use tracing::{Instrument, Level, Span, error, info, info_span};

use crate::bg_tasks::{WriteJob, spawn_writer_task};
use crate::configuration::Settings;
use crate::formatter::ColoredOnResponse;
use crate::routes::{
    add_favoritos, auth, delete_favoritos, delete_historial, documents, favoritos, health_check,
    historial_detailed, historial_summary, search, serve_ui,
};
use crate::search::SearchStrategy;

#[derive(Debug, Clone)]
pub struct ServerState {
    pub documents: Vec<Document>,
    pub writer: UnboundedSender<WriteJob>,
    pub embeddings_provider: OpenAIClient,
    pub pool: AsyncConnectionPool,
    pub embeddings_cache: Cache<String, Arc<Vec<f32>>>,
}

#[derive(Debug)]
pub enum CacheResult<T> {
    Hit(T),
    Miss(T),
    Skip,
}

impl<T> CacheResult<T> {
    pub fn into_inner(self) -> Option<T> {
        match self {
            CacheResult::Hit(value) | CacheResult::Miss(value) => Some(value),
            CacheResult::Skip => None,
        }
    }

    pub fn is_hit(&self) -> bool {
        matches!(self, CacheResult::Hit(_))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("Embedding generation failed: {0}")]
    EmbeddingError(String),
    #[error("Cache operation failed: {0}")]
    CacheError(String),
}

impl ServerState {
    pub async fn get_embeddings(
        &self,
        query: &str,
        client: &Client,
        strategy: SearchStrategy,
        span: &Span,
    ) -> Result<CacheResult<Arc<Vec<f32>>>, CacheError> {
        match strategy {
            SearchStrategy::Semantic | SearchStrategy::ReciprocalRankFusion => {
                if let Some(cached_embedding) = self.embeddings_cache.get(query).await {
                    span.record("source", "hit");
                    return Ok(CacheResult::Hit(cached_embedding));
                }

                let embedding_span = info_span!("embedding.request");
                let _guard = embedding_span.enter();

                let embedding = Arc::new(
                    self.embeddings_provider
                        // TODO: Add support for retries
                        .embed_single(query, client)
                        .await
                        .map_err(|e| CacheError::EmbeddingError(e.to_string()))?,
                );

                self.embeddings_cache
                    .insert(query.to_string(), embedding.clone())
                    .await;

                span.record("source", "miss");
                Ok(CacheResult::Miss(embedding))
            }
            SearchStrategy::Fts => {
                span.record("source", "dynamic");
                Ok(CacheResult::Skip)
            }
        }
    }
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

                            info_span!(
                                "request",
                                id = %request_id,
                                trace_id = tracing::field::Empty,
                                span_id = tracing::field::Empty,
                                method = %request.method().blue().bold(),
                                uri = %request.uri(),
                            )
                        })
                        .on_request(|_request: &Request<Body>, span: &Span| {
                            let context = span.context();

                            let span_id = context.span().span_context().span_id().to_string();
                            let trace_id = context.span().span_context().trace_id().to_string();

                            span.record("span_id", &span_id);
                            span.record("trace_id", &trace_id);
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
            dbg!("{:?}", &configuration);
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
