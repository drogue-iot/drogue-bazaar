use crate::core::config::ConfigFromEnv;

/// Initialize and run an application.
///
/// ```
/// use drogue_bazaar::{app, project};
///
/// project!("Drogue IoT");
///
/// #[derive(Debug, serde::Deserialize)]
/// struct Config {
///     my_config_value: String,
///     my_config_flag: bool,
/// }
///
/// async fn run(config: Config) -> anyhow::Result<()> {
///     Ok(())
/// }
///
/// async fn _main() -> anyhow::Result<()> {
///     app!(run)
/// }
/// ```
#[macro_export]
macro_rules! app {
    ($f:ident) => {
        $crate::app!(PROJECT, $f)
    };
    ($project:expr, $f:ident) => {
        $crate::main!($project, $crate::macros::run_app($f)?)
    };
}

#[doc(hidden)]
pub fn run_app<C, F, Fut>(f: F) -> anyhow::Result<Fut>
where
    for<'de> C: ConfigFromEnv<'de>,
    F: FnOnce(C) -> Fut,
    Fut: core::future::Future<Output = anyhow::Result<()>>,
{
    let cfg = C::from_env()?;
    Ok(f(cfg))
}

/// Initialize an application stack and run the provided future to completion.
///
/// ```
/// use drogue_bazaar::{main, project};
///
/// project!("Drogue IoT");
///
/// async fn run() {}
///
/// async fn _main() {
///     main!(PROJECT, run())
/// }
/// ```
#[macro_export]
macro_rules! main {
    ($project:expr, $run:expr) => {{

        use std::io::Write;

        $crate::component!(COMPONENT, &$project);

        use $crate::core::config::ConfigFromEnv;

        $crate::app::init::phase1();

        println!(r#"______ ______  _____  _____  _   _  _____   _____         _____ 
|  _  \| ___ \|  _  ||  __ \| | | ||  ___| |_   _|       |_   _|
| | | || |_/ /| | | || |  \/| | | || |__     | |    ___    | |  
| | | ||    / | | | || | __ | | | ||  __|    | |   / _ \   | |  
| |/ / | |\ \ \ \_/ /| |_\ \| |_| || |___   _| |_ | (_) |  | |  
|___/  \_| \_| \___/  \____/ \___/ \____/   \___/  \___/   \_/  
{} {} - {} {} ({})
"#, COMPONENT.project.name, COMPONENT.project.version, COMPONENT.name, COMPONENT.version, COMPONENT.description);

        std::io::stdout().flush().ok();

        $crate::app::init::phase2(COMPONENT.name);

        $run.await
    }};
}
