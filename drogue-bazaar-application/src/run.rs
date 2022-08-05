use drogue_bazaar_core::Spawner;
use futures_core::future::LocalBoxFuture;
use futures_util::stream::FuturesUnordered;
use std::future::Future;
use std::pin::Pin;

/// A main runner.
pub struct Main<'m> {
    tasks: FuturesUnordered<LocalBoxFuture<'m, anyhow::Result<()>>>,

    #[cfg(feature = "health")]
    health_config: Option<crate::health::HealthServerConfig>,
    #[cfg(feature = "health")]
    health_checks: Vec<Box<dyn crate::health::HealthChecked>>,
}

impl<'m> Extend<LocalBoxFuture<'m, Result<(), anyhow::Error>>> for Main<'m> {
    fn extend<T: IntoIterator<Item = LocalBoxFuture<'m, anyhow::Result<()>>>>(&mut self, iter: T) {
        self.tasks.extend(iter)
    }
}

impl<'m> Main<'m> {
    pub fn new(
        #[cfg(feature = "health")] health_config: Option<crate::health::HealthServerConfig>,
    ) -> Self {
        Self {
            tasks: Default::default(),
            #[cfg(feature = "health")]
            health_config,
            #[cfg(feature = "health")]
            health_checks: Vec::new(),
        }
    }

    /// Add tasks to run.
    pub fn add<I>(mut self, tasks: I) -> Self
    where
        I: IntoIterator<Item = LocalBoxFuture<'m, anyhow::Result<()>>>,
    {
        self.extend(tasks);
        self
    }

    #[cfg(feature = "health")]
    pub fn add_health<I>(&mut self, i: I)
    where
        I: IntoIterator<Item = Box<dyn crate::health::HealthChecked>>,
    {
        self.health_checks.extend(i);
    }

    pub async fn run(#[allow(unused_mut)] mut self) -> anyhow::Result<()> {
        #[cfg(feature = "console-metrics")]
        self.add_console_metrics();

        #[cfg(feature = "health")]
        self.add_health_server();

        let (result, _, _) = futures_util::future::select_all(self.tasks).await;

        log::warn!("One of the main runners returned: {result:?}");
        log::warn!("Exiting application...");

        Ok(())
    }

    #[cfg(feature = "health")]
    fn add_health_server(&mut self) {
        if let Some(health) = self.health_config.take() {
            let checks = std::mem::take(&mut self.health_checks);

            let health = crate::health::HealthServer::new(
                health,
                checks,
                Some(prometheus::default_registry().clone()),
            );

            self.tasks.push(Box::pin(health.run()));
        }
    }

    #[cfg(feature = "console-metrics")]
    fn add_console_metrics(&mut self) {
        use futures_util::future::FutureExt;
        use prometheus::{Encoder, TextEncoder};
        use std::time::Duration;

        self.tasks.push(
            async move {
                log::info!("Starting console metrics loop...");
                let encoder = TextEncoder::new();
                loop {
                    let metric_families = prometheus::gather();
                    {
                        let mut out = std::io::stdout().lock();
                        encoder.encode(&metric_families, &mut out).unwrap();
                    }
                    tokio::time::sleep(Duration::from_secs(60)).await;
                }
            }
            .boxed_local(),
        );
    }
}

impl<'m> Spawner<anyhow::Result<()>> for Main<'m> {
    fn spawn(&mut self, future: Pin<Box<dyn Future<Output = anyhow::Result<()>>>>) {
        self.tasks.push(future);
    }
}
