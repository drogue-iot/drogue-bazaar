use super::{HealthChecker, HealthServerConfig};
use crate::health::HealthChecked;
use prometheus::Registry;
use serde_json::json;

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

    pub async fn run(self) -> anyhow::Result<()> {
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

        actix_web::HttpServer::new(move || {
            use actix_web::App;

            health_app!(checker, app_data).wrap(prometheus.clone())
        })
        .bind(self.config.bind_addr)?
        .workers(self.config.workers)
        .run()
        .await?;

        Ok(())
    }
}
