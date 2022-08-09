use crate::{
    app::{
        health::{HealthChecker, HealthServer},
        RuntimeConfig, Startup,
    },
    core::{config::ConfigFromEnv, Spawner},
    health::HealthChecked,
};
use futures_core::future::LocalBoxFuture;
use futures_util::future::FutureExt;
use humantime::format_duration;
use prometheus::{Encoder, TextEncoder};
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;

/// A main runner.
///
/// The idea of the main runner is to perform all setup steps, gathering all tasks (futures) to be
/// executed, and then initialize the stack and drive the tasks, until one of them completes.
///
/// In some cases it might be necessary to run a set of tasks on a different context (like actix, or
/// ntex). In this case it is possible to create a [`SubMain`] instance using [`SubMain::sub_main`].
pub struct Main<'m> {
    sub: SubMain<'m>,
}

impl<'m> Default for Main<'m> {
    fn default() -> Self {
        Self::new(RuntimeConfig::default())
    }
}

impl<'m> Deref for Main<'m> {
    type Target = SubMain<'m>;

    fn deref(&self) -> &Self::Target {
        &self.sub
    }
}

impl<'m> DerefMut for Main<'m> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sub
    }
}

impl<'m> Main<'m> {
    pub fn new(config: RuntimeConfig) -> Self {
        Self {
            sub: SubMain::new(config, Default::default()),
        }
    }

    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self::new(RuntimeConfig::from_env_prefix("RUNTIME__")?))
    }

    /// Add tasks to run.
    pub fn add_tasks<I>(mut self, tasks: I) -> Self
    where
        I: IntoIterator<Item = LocalBoxFuture<'m, anyhow::Result<()>>>,
    {
        self.extend(tasks);
        self
    }

    pub fn add_checks<I>(&mut self, i: I)
    where
        I: IntoIterator<Item = Box<dyn HealthChecked>>,
    {
        self.sub.health.extend(i);
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        self.run_console_metrics();
        self.run_health_server();

        self.sub.run().await
    }

    fn run_health_server(&mut self) {
        if self.config.health.enabled {
            let health = HealthServer::new(
                self.config.health.clone(),
                self.health.clone(),
                Some(prometheus::default_registry().clone()),
            );

            self.tasks.push(health.run().boxed());
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
                .boxed(),
            );
        }
    }
}

impl Spawner for Main<'_> {
    fn spawn_boxed(&mut self, future: Pin<Box<dyn Future<Output = anyhow::Result<()>>>>) {
        SubMain::spawn_boxed(self, future)
    }
}

impl Startup for Main<'_> {
    fn check_boxed(&mut self, check: Box<dyn HealthChecked>) {
        SubMain::check_boxed(self, check)
    }

    fn use_tracing(&self) -> bool {
        SubMain::use_tracing(self)
    }

    fn runtime_config(&self) -> &RuntimeConfig {
        SubMain::runtime_config(self)
    }
}

/// A sub-main instance, which can be used to contribute global tasks to the main instance which
/// created this sub instance, but gather own tasks, which can be run independently by calling
/// the [`SubMain::run`] function.
pub struct SubMain<'m> {
    config: RuntimeConfig,
    tasks: Vec<LocalBoxFuture<'m, anyhow::Result<()>>>,
    health: HealthChecker,
}

impl SubMain<'_> {
    pub(crate) fn new(config: RuntimeConfig, health: HealthChecker) -> Self {
        Self {
            config,
            tasks: Default::default(),
            health,
        }
    }

    /// Returns `true` is there are no tasks scheduled so far.
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Create a new sub-main instance.
    pub fn sub_main(&self) -> SubMain {
        self.sub_main_seed().into()
    }

    /// Create a seed or a sub-main instance, which can be sent.
    pub fn sub_main_seed(&self) -> SubMainSeed {
        SubMainSeed::new(self.config.clone(), self.health.clone())
    }

    /// Run the recorded tasks.
    ///
    /// **NOTE:** This does not run any health checks, these must be run by the main instance.
    pub async fn run(self) -> anyhow::Result<()> {
        let (result, _, _) = futures_util::future::select_all(self.tasks).await;

        log::warn!("One of the main runners returned: {result:?}");
        log::warn!("Exiting application...");

        Ok(())
    }
}

impl<'m> Extend<LocalBoxFuture<'m, Result<(), anyhow::Error>>> for SubMain<'m> {
    fn extend<T: IntoIterator<Item = LocalBoxFuture<'m, anyhow::Result<()>>>>(&mut self, iter: T) {
        self.tasks.extend(iter)
    }
}

impl<'m> Spawner for SubMain<'m> {
    fn spawn_boxed(&mut self, future: Pin<Box<dyn Future<Output = anyhow::Result<()>>>>) {
        self.tasks.push(future);
    }
}

impl<'m> Startup for SubMain<'m> {
    fn check_boxed(&mut self, check: Box<dyn HealthChecked>) {
        self.health.push(check);
    }

    fn use_tracing(&self) -> bool {
        self.config.tracing.is_enabled()
    }

    fn runtime_config(&self) -> &RuntimeConfig {
        &self.config
    }
}

/// A seed for a [`SubMain`] instance.
///
/// As the tasks in a `SubMain` are not [`Send`], it is no possible to send the sub instance. Which
/// may be required it is should be executed on another scheduler. For this it is possible to
/// create a "seed", which can later (after sending) be turned into a proper instance.
pub struct SubMainSeed {
    config: RuntimeConfig,
    health: HealthChecker,
}

impl SubMainSeed {
    fn new(config: RuntimeConfig, health: HealthChecker) -> Self {
        Self { config, health }
    }
}

impl From<SubMainSeed> for SubMain<'_> {
    fn from(seed: SubMainSeed) -> Self {
        Self {
            config: seed.config,
            health: seed.health,
            tasks: Default::default(),
        }
    }
}
