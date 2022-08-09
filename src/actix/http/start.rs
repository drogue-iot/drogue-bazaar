use super::{bind::bind_http, config::HttpConfig};
use crate::app::{Startup, StartupExt};
use crate::{
    app::RuntimeConfig,
    core::tls::{TlsMode, WithTlsMode},
};
use actix_cors::Cors;
use actix_http::Extensions;
use actix_web::{
    middleware,
    web::{self, ServiceConfig},
    App, HttpServer,
};
use actix_web_extras::middleware::Condition;
use futures_core::future::LocalBoxFuture;
use futures_util::{FutureExt, TryFutureExt};
use std::{any::Any, sync::Arc};

/// Build a CORS setup.
#[derive(Clone)]
pub enum CorsBuilder {
    Disabled,
    Permissive,
    Custom(Arc<dyn Fn() -> Cors + Send + Sync>),
}

impl Default for CorsBuilder {
    fn default() -> Self {
        Self::Disabled
    }
}

impl<F> From<F> for CorsBuilder
where
    F: Fn() -> Cors + Send + Sync + 'static,
{
    fn from(f: F) -> Self {
        CorsBuilder::Custom(Arc::new(f))
    }
}

pub type OnConnectFn = dyn Fn(&dyn Any, &mut Extensions) + Send + Sync + 'static;

/// Build an HTTP server.
pub struct HttpBuilder<F>
where
    F: Fn(&mut ServiceConfig) + Send + Clone + 'static,
{
    config: HttpConfig,
    app_builder: Box<F>,
    cors_builder: CorsBuilder,
    on_connect: Option<Box<OnConnectFn>>,
    tls_mode: TlsMode,
    tracing: bool,
}

impl<F> HttpBuilder<F>
where
    F: Fn(&mut ServiceConfig) + Send + Clone + 'static,
{
    /// Start building a new HTTP server instance.
    pub fn new(config: HttpConfig, runtime: Option<&RuntimeConfig>, app_builder: F) -> Self {
        Self {
            config,
            app_builder: Box::new(app_builder),
            cors_builder: Default::default(),
            on_connect: None,
            tls_mode: TlsMode::NoClient,
            tracing: runtime.map(|r| r.tracing.is_enabled()).unwrap_or_default(),
        }
    }

    /// Set the CORS builder.
    pub fn cors<I: Into<CorsBuilder>>(mut self, cors_builder: I) -> Self {
        self.cors_builder = cors_builder.into();
        self
    }

    /// Set an "on connect" handler.
    pub fn on_connect<O>(mut self, on_connect: O) -> Self
    where
        O: Fn(&dyn Any, &mut Extensions) + Send + Sync + 'static,
    {
        self.on_connect = Some(Box::new(on_connect));
        self
    }

    /// Set the TLS mode.
    pub fn tls_mode<I: Into<TlsMode>>(mut self, tls_mode: I) -> Self {
        self.tls_mode = tls_mode.into();
        self
    }

    /// Start the server on the provided startup context.
    pub fn start(self, startup: &mut dyn Startup) -> anyhow::Result<()> {
        startup.spawn(self.run()?);
        Ok(())
    }

    /// Run the server.
    ///
    /// **NOTE:** This only returns a future, which was to be scheduled on some executor. Possibly
    /// using [`crate::app::Startup`].
    ///
    /// In most cases you want to use [`Self::start`] instead.
    pub fn run(self) -> Result<LocalBoxFuture<'static, Result<(), anyhow::Error>>, anyhow::Error> {
        let max_payload_size = self.config.max_payload_size;
        let max_json_payload_size = self.config.max_json_payload_size;

        let prometheus = actix_web_prom::PrometheusMetricsBuilder::new(
            self.config.metrics_namespace.as_deref().unwrap_or("drogue"),
        )
        .registry(prometheus::default_registry().clone())
        .build()
        // FIXME: replace with direct conversion once nlopes/actix-web-prom#67 is merged
        .map_err(|err| anyhow::anyhow!("Failed to build prometheus middleware: {err}"))?;

        let mut main = HttpServer::new(move || {
            let cors = match self.cors_builder.clone() {
                CorsBuilder::Disabled => None,
                CorsBuilder::Permissive => Some(Cors::permissive()),
                CorsBuilder::Custom(f) => Some(f()),
            };

            let (logger, tracing_logger) = match self.tracing {
                false => (Some(middleware::Logger::default()), None),
                true => (None, Some(tracing_actix_web::TracingLogger::default())),
            };

            let app = App::new();

            // add wrapper (the last added is executed first)

            // enable CORS support
            let app = app.wrap(Condition::from_option(cors));

            // record request metrics
            let app = app.wrap(prometheus.clone());

            let app = app
                .wrap(Condition::from_option(logger))
                // logging: ... other tracing
                .wrap(Condition::from_option(tracing_logger));

            // configure payload and JSON payload limits
            let app = app
                .app_data(web::PayloadConfig::new(max_payload_size))
                .app_data(web::JsonConfig::default().limit(max_json_payload_size));

            // configure main http application
            app.configure(|cfg| (self.app_builder)(cfg))
        });

        if let Some(on_connect) = self.on_connect {
            main = main.on_connect(on_connect);
        }

        let mut main = bind_http(
            main,
            self.config.bind_addr,
            self.config.disable_tls.with_tls_mode(self.tls_mode),
            self.config.key_file,
            self.config.cert_bundle_file,
        )?;

        if let Some(workers) = self.config.workers {
            main = main.workers(workers)
        }

        Ok(main.run().err_into().boxed_local())
    }
}