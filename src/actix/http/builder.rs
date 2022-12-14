use super::{bind::bind_http, config::HttpConfig};
use crate::actix::http::{BuildCors, CorsConfig};
use crate::app::{Startup, StartupExt};
use crate::{
    app::RuntimeConfig,
    core::tls::{TlsAuthConfig, WithTlsAuthConfig},
};
use actix_cors::Cors;
use actix_http::Extensions;
use actix_web::{
    middleware,
    web::{self, ServiceConfig},
    App, HttpServer,
};
use actix_web_extras::middleware::Condition;
use futures_core::future::BoxFuture;
use futures_util::{FutureExt, TryFutureExt};
use std::any::Any;

pub type OnConnectFn = dyn Fn(&dyn Any, &mut Extensions) + Send + Sync + 'static;

/// Build an HTTP server.
pub struct HttpBuilder<F>
where
    F: Fn(&mut ServiceConfig) + Send + Clone + 'static,
{
    config: HttpConfig,
    default_cors: Option<CorsConfig>,
    app_builder: Box<F>,
    on_connect: Option<Box<OnConnectFn>>,
    tls_auth_config: TlsAuthConfig,
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
            default_cors: None,
            app_builder: Box::new(app_builder),
            on_connect: None,
            tls_auth_config: TlsAuthConfig::default(),
            tracing: runtime.map(|r| r.tracing.is_enabled()).unwrap_or_default(),
        }
    }

    /// Set a default CORS config without overriding the existing one.
    pub fn default_cors<C: Into<Option<CorsConfig>>>(mut self, default_cors: C) -> Self {
        self.default_cors = default_cors.into();
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
    pub fn tls_auth_config<I: Into<TlsAuthConfig>>(mut self, tls_auth_config: I) -> Self {
        self.tls_auth_config = tls_auth_config.into();
        self
    }

    /// Start the server on the provided startup context.
    pub fn start(self, startup: &mut dyn Startup) -> anyhow::Result<()> {
        startup.spawn(self.run()?);
        Ok(())
    }

    /// Get the effective CORS config.
    ///
    /// This will either the configuration provided through the [`HttpConfig`], or the one
    /// registered by the application using [`Self::default_cors()`]
    fn cors_config(&self) -> Option<CorsConfig> {
        self.config
            .cors
            .as_ref()
            .or(self.default_cors.as_ref())
            .cloned()
    }

    /// Run the server.
    ///
    /// **NOTE:** This only returns a future, which was to be scheduled on some executor. Possibly
    /// using [`crate::app::Startup`].
    ///
    /// In most cases you want to use [`Self::start`] instead.
    pub fn run(
        #[allow(unused_mut)] mut self,
    ) -> Result<BoxFuture<'static, Result<(), anyhow::Error>>, anyhow::Error> {
        let max_payload_size = self.config.max_payload_size;
        let max_json_payload_size = self.config.max_json_payload_size;

        let prometheus = actix_web_prom::PrometheusMetricsBuilder::new(
            self.config.metrics_namespace.as_deref().unwrap_or("drogue"),
        )
        .registry(prometheus::default_registry().clone())
        .build()
        // FIXME: replace with direct conversion once nlopes/actix-web-prom#67 is merged
        .map_err(|err| anyhow::anyhow!("Failed to build prometheus middleware: {err}"))?;

        let cors = self.cors_config();
        log::debug!("Effective CORS config {cors:?}");

        // we just try to parse it once, so we can be sure it doesn't panic later
        let _: Option<Cors> = cors.build_cors()?;

        let mut main = HttpServer::new(move || {
            let app = App::new();

            // add wrapper (the last added is executed first)

            // enable CORS support
            // this should not panic, as we did parse the configuration once, before the http builder
            let cors: Option<Cors> = cors.build_cors().expect("Configuration must be valid");
            let app = app.wrap(Condition::from_option(cors));

            // record request metrics
            let app = app.wrap(prometheus.clone());

            // request logging
            let (logger, tracing_logger) = match self.tracing {
                false => (Some(middleware::Logger::default()), None),
                true => (None, Some(tracing_actix_web::TracingLogger::default())),
            };
            log::debug!(
                "Loggers ({}) - logger: {}, tracing: {}",
                self.tracing,
                logger.is_some(),
                tracing_logger.is_some()
            );
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

        if self.config.disable_tls_psk {
            #[cfg(feature = "openssl")]
            self.tls_auth_config.psk.take();
        }

        let mut main = bind_http(
            main,
            self.config.bind_addr,
            self.config
                .disable_tls
                .with_tls_auth_config(self.tls_auth_config),
            self.config.key_file,
            self.config.cert_bundle_file,
        )?;

        if let Some(workers) = self.config.workers {
            main = main.workers(workers)
        }

        Ok(main.run().err_into().boxed())
    }
}
