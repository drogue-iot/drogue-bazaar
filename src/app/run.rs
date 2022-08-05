use crate::app::init;
use crate::app::init::Tracing;
use crate::core::config::ConfigFromEnv;
use crate::core::info::ComponentInformation;
use crate::health::HealthChecked;
use crate::{
    app::health::{HealthServer, HealthServerConfig},
    core::Spawner,
};
use futures_core::future::LocalBoxFuture;
use futures_util::future::FutureExt;
use humantime::format_duration;
use prometheus::{Encoder, TextEncoder};
use std::future::Future;
use std::io::Write;
use std::pin::Pin;
use std::time::Duration;

#[derive(Debug, Default, serde::Deserialize)]
pub struct RuntimeConfig {
    #[serde(default)]
    pub console_metrics: ConsoleMetrics,
    #[serde(default)]
    pub health: HealthServerConfig,
    #[serde(default)]
    pub tracing: Tracing,
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

pub struct Runtime {
    component: ComponentInformation,
    dotenv: Option<bool>,
    show_banner: Option<bool>,
}

/// Create a new runtime, using the local crate a component.
///
/// ```
/// use drogue_bazaar::{project, runtime, app::{Main, Startup}};
///
/// project!(PROJECT: "Drogue IoT");
///
/// #[derive(serde::Deserialize)]
/// struct Config {}
///
/// async fn run(config: Config, startup: &mut dyn Startup) -> anyhow::Result<()> {
///     Ok(())
/// }
///
/// fn main() {
///     let runtime = runtime!(PROJECT)
///         .exec(run);
/// }
/// ```
#[macro_export]
macro_rules! runtime {
    ($project:expr) => {
        $crate::app::Runtime::new($crate::component!($project))
    };
}

impl Runtime {
    pub fn new(component: ComponentInformation) -> Self {
        Self {
            component,
            dotenv: None,
            show_banner: None,
        }
    }

    /// Force dotenv option.
    ///
    /// ```
    /// use drogue_bazaar::{project, runtime};
    ///
    /// project!(PROJECT: "Drogue IoT");
    ///
    /// fn main() {
    ///     runtime!(PROJECT)
    ///         .dotenv(false);
    /// }
    /// ```
    pub fn dotenv<I: Into<Option<bool>>>(mut self, dotenv: I) -> Self {
        self.dotenv = dotenv.into();
        self
    }

    /// Show the application banner
    fn banner(&self) {
        if self
            .show_banner
            .or_else(|| flag_opt("RUNTIME__SHOW_BANNER"))
            .unwrap_or(true)
        {
            println!(
                r#"{}  
{} {} - {} {} ({})
"#,
                self.component.project.banner,
                self.component.project.name,
                self.component.project.version,
                self.component.name,
                self.component.version,
                self.component.description
            );

            std::io::stdout().flush().ok();
        }
    }

    pub async fn exec<C, A>(self, app: A) -> anyhow::Result<()>
    where
        A: App<C>,
        for<'de> C: ConfigFromEnv<'de>,
    {
        // phase 1: early init, cannot really rely on env-vars, but may add its own

        init::phase1(
            self.dotenv
                .unwrap_or_else(|| !flag("RUNTIME__DISABLE_DOTENV")),
        );

        // phase 2: Show early runtime information
        self.banner();

        // phase 3: env-vars are ready now, we can make use of them

        let mut main = Main::from_env()?;

        init::phase2(self.component.name, main.config.tracing.clone());

        // phase 4: main app startup

        let config = C::from_env()?;
        app.run(config, &mut main).await?;
        main.run().await?;

        // done

        Ok(())
    }

    pub async fn exec_fn<C, F>(self, f: F) -> anyhow::Result<()>
    where
        for<'de> C: ConfigFromEnv<'de> + Send + 'static,
        F: for<'f> AppFn<C, &'f mut dyn Startup>,
    {
        self.exec(f).await
    }
}

pub trait AppFn<C, S>: FnOnce(C, S) -> <Self as AppFn<C, S>>::Fut {
    type Fut: Future<Output = anyhow::Result<()>>;
}

impl<C, S, F, Fut> AppFn<C, S> for F
where
    F: FnOnce(C, S) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    type Fut = Fut;
}

#[async_trait::async_trait(?Send)]
pub trait App<C>
where
    for<'de> C: ConfigFromEnv<'de>,
{
    async fn run(self, config: C, startup: &mut dyn Startup) -> anyhow::Result<()>;
}

#[async_trait::async_trait(?Send)]
impl<C, A> App<C> for A
where
    A: for<'f> AppFn<C, &'f mut dyn Startup>,
    C: for<'de> ConfigFromEnv<'de> + 'static,
{
    async fn run(self, config: C, startup: &mut dyn Startup) -> anyhow::Result<()> {
        (self)(config, startup).await
    }
}

/// A main runner.
pub struct Main<'m> {
    config: RuntimeConfig,

    tasks: Vec<LocalBoxFuture<'m, anyhow::Result<()>>>,

    health_checks: Vec<Box<dyn HealthChecked>>,
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

            let task = health.run();

            self.tasks.push(task.boxed());
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

impl<'m> Spawner for Main<'m> {
    fn spawn_boxed(&mut self, future: Pin<Box<dyn Future<Output = anyhow::Result<()>>>>) {
        self.tasks.push(future);
    }
}

/// Startup context.
pub trait Startup: Spawner {
    /// Add a health check.
    fn check(&mut self, check: Box<dyn HealthChecked>);

    /// Allow the application to check if the runtime wants to enable tracing.
    ///
    /// This can be used to e.g. add some tracing logger into the HTTP stack.
    fn use_tracing(&self) -> bool;
}

impl<'m> Startup for Main<'m> {
    fn check(&mut self, check: Box<dyn HealthChecked>) {
        self.health_checks.push(check);
    }

    fn use_tracing(&self) -> bool {
        self.config.tracing.is_enabled()
    }
}

pub trait StartupExt: Startup {
    /// Add several health checks at once.
    fn check_iter<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Box<dyn HealthChecked>>,
    {
        for i in iter {
            self.check(i);
        }
    }

    fn spawn_iter<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Pin<Box<dyn Future<Output = anyhow::Result<()>>>>>,
    {
        for i in iter {
            self.spawn_boxed(i);
        }
    }

    fn spawn<F>(&mut self, f: F)
    where
        F: Future<Output = anyhow::Result<()>> + 'static,
    {
        self.spawn_boxed(Box::pin(f))
    }
}

impl<S: ?Sized> StartupExt for S where S: Startup {}

fn flag(name: &str) -> bool {
    flag_opt(name).unwrap_or_default()
}

fn flag_opt(name: &str) -> Option<bool> {
    std::env::var(name).map(|v| v.to_lowercase() == "true").ok()
}
