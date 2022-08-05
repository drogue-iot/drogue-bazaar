use crate::core::config::ConfigFromEnv;
use crate::{
    app::health::{HealthServer, HealthServerConfig},
    core::Spawner,
};
use futures_core::future::LocalBoxFuture;
use futures_util::{future::FutureExt, stream::FuturesUnordered};
use humantime::format_duration;
use prometheus::{Encoder, TextEncoder};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

#[derive(Debug, Default, serde::Deserialize)]
pub struct RuntimeConfig {
    #[serde(default)]
    pub console_metrics: ConsoleMetrics,
    #[serde(default)]
    pub health: HealthServerConfig,
}

#[derive(Debug, serde::Deserialize)]
pub struct ConsoleMetrics {
    pub enabled: bool,
    #[serde(
        default = "default::console_metrics_duration",
        with = "humantime_serde"
    )]
    pub period: Duration,
}

impl Default for ConsoleMetrics {
    fn default() -> Self {
        Self {
            enabled: false,
            period: default::console_metrics_duration(),
        }
    }
}

mod default {
    use super::*;

    pub const fn console_metrics_duration() -> Duration {
        Duration::from_secs(60)
    }
}

/// A main runner.
pub struct Main<'m> {
    config: RuntimeConfig,

    tasks: FuturesUnordered<LocalBoxFuture<'m, anyhow::Result<()>>>,

    health_checks: Vec<Box<dyn crate::health::HealthChecked>>,
}

impl<'m> Extend<LocalBoxFuture<'m, Result<(), anyhow::Error>>> for Main<'m> {
    fn extend<T: IntoIterator<Item = LocalBoxFuture<'m, anyhow::Result<()>>>>(&mut self, iter: T) {
        self.tasks.extend(iter)
    }
}

impl<'m> Default for Main<'m> {
    fn default() -> Self {
        Self::new(RuntimeConfig::default())
    }
}

impl<'m> Main<'m> {
    pub fn new(config: RuntimeConfig) -> Self {
        Self {
            config,

            tasks: Default::default(),
            health_checks: Vec::new(),
        }
    }

    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self::new(RuntimeConfig::from_env_prefix("RUNTIME__")?))
    }

    /// Add tasks to run.
    pub fn add<I>(mut self, tasks: I) -> Self
    where
        I: IntoIterator<Item = LocalBoxFuture<'m, anyhow::Result<()>>>,
    {
        self.extend(tasks);
        self
    }

    pub fn add_health<I>(&mut self, i: I)
    where
        I: IntoIterator<Item = Box<dyn crate::health::HealthChecked>>,
    {
        self.health_checks.extend(i);
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        self.run_console_metrics();
        self.run_health_server();

        let (result, _, _) = futures_util::future::select_all(self.tasks).await;

        log::warn!("One of the main runners returned: {result:?}");
        log::warn!("Exiting application...");

        Ok(())
    }

    fn run_health_server(&mut self) {
        if self.config.health.enabled {
            let checks = std::mem::take(&mut self.health_checks);

            let health = HealthServer::new(
                self.config.health.clone(),
                checks,
                Some(prometheus::default_registry().clone()),
            );

            self.tasks.push(Box::pin(health.run()));
        }
    }

    fn run_console_metrics(&mut self) {
        if self.config.console_metrics.enabled {
            let period = self.config.console_metrics.period;

            self.tasks.push(
                async move {
                    log::info!(
                        "Starting console metrics loop ({})...",
                        format_duration(period)
                    );
                    let encoder = TextEncoder::new();
                    loop {
                        let metric_families = prometheus::gather();
                        {
                            let mut out = std::io::stdout().lock();
                            encoder.encode(&metric_families, &mut out).unwrap();
                        }
                        tokio::time::sleep(period).await;
                    }
                }
                .boxed_local(),
            );
        }
    }
}

impl<'m> Spawner<anyhow::Result<()>> for Main<'m> {
    fn spawn(&mut self, future: Pin<Box<dyn Future<Output = anyhow::Result<()>>>>) {
        self.tasks.push(future);
    }
}
