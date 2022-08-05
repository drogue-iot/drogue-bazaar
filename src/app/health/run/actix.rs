use super::{HealthChecker, HealthServerConfig};
use crate::health::HealthChecked;
use anyhow::anyhow;
use futures_util::{future::err, TryFutureExt};
use prometheus::Registry;
use serde_json::json;
use std::{future::Future, pin::Pin};

/// A server, running health check endpoints.
pub struct HealthServer {
    config: HealthServerConfig,
    checker: HealthChecker,
    registry: Option<Registry>,
}

macro_rules! health_endpoint {
    ($sys:ident) => {
        async fn index() -> $sys::HttpResponse {
            $sys::HttpResponse::Ok().json(&json!({}))
        }

        async fn readiness(checker: Data<HealthChecker>) -> $sys::HttpResponse {
            let (code, body) = super::run_checks(checker.into_inner(), |checker| async move {
                checker.is_ready().await
            })
            .await;
            $sys::HttpResponse::build(code.into()).json(&body)
        }

        async fn liveness(checker: Data<HealthChecker>) -> $sys::HttpResponse {
            let (code, body) = super::run_checks(checker.into_inner(), |checker| async move {
                checker.is_alive().await
            })
            .await;
            $sys::HttpResponse::build(code.into()).json(&body)
        }
    };
}

macro_rules! health_app {
    ($checker:expr, $app_data:ident) => {
        App::new()
            .$app_data($checker.clone())
            .route("/", web::get().to(index))
            .route("/readiness", web::get().to(readiness))
            .route("/liveness", web::get().to(liveness))
    };
}

impl HealthServer {
    pub fn new(
        config: HealthServerConfig,
        checks: Vec<Box<dyn HealthChecked>>,
        registry: Option<Registry>,
    ) -> Self {
        Self {
            config,
            checker: HealthChecker { checks },
            registry,
        }
    }

    pub fn run(self) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>> {
        use actix_web::web;
        use actix_web::web::Data;
        health_endpoint!(actix_web);

        let checker = Data::new(self.checker);

        let prometheus = match self.registry {
            Some(metrics) => actix_web_prom::PrometheusMetricsBuilder::new("health")
                .registry(metrics)
                .endpoint("/metrics")
                .build()
                .unwrap(),
            _ => actix_web_prom::PrometheusMetricsBuilder::new("noop")
                .build()
                .unwrap(),
        };

        let http = actix_web::HttpServer::new(move || {
            use actix_web::App;

            health_app!(checker, app_data).wrap(prometheus.clone())
        });

        let http = match http.bind(self.config.bind_addr) {
            Ok(http) => http,
            Err(e) => return Box::pin(err(anyhow!(e))),
        };

        let task = http
            .workers(self.config.workers)
            .run()
            .map_err(|err| anyhow!(err));

        Box::pin(task)
    }
}
