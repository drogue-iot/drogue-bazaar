#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Tracing {
    Disabled,
    Jaeger,
}

impl Default for Tracing {
    fn default() -> Self {
        Self::Disabled
    }
}

impl Tracing {
    /// Check if tracing is enabled, or not.
    pub fn is_enabled(&self) -> bool {
        !matches!(self, Self::Disabled)
    }
}

/// Try getting the sampling rate from the environment variables
fn sampling_from_env() -> Option<f64> {
    std::env::var_os("OTEL_TRACES_SAMPLER_ARG")
        .and_then(|s| s.to_str().map(|s| s.parse::<f64>().ok()).unwrap())
}

fn sampler() -> opentelemetry::sdk::trace::Sampler {
    if let Some(p) = sampling_from_env() {
        opentelemetry::sdk::trace::Sampler::TraceIdRatioBased(p)
    } else {
        opentelemetry::sdk::trace::Sampler::TraceIdRatioBased(0.001)
    }
}

pub fn init_tracing(name: &str, tracing: Tracing) {
    match tracing {
        Tracing::Disabled => {
            init_no_tracing();
        }
        Tracing::Jaeger => {
            init_jaeger(name);
        }
    }
}

pub fn init_jaeger(name: &str) {
    use tracing_subscriber::prelude::*;

    opentelemetry::global::set_text_map_propagator(
        opentelemetry::sdk::propagation::TraceContextPropagator::new(),
    );
    let pipeline = opentelemetry_jaeger::new_pipeline()
        .with_service_name(name)
        .with_trace_config(opentelemetry::sdk::trace::Config::default().with_sampler(
            opentelemetry::sdk::trace::Sampler::ParentBased(Box::new(sampler())),
        ));

    println!("Using Jaeger tracing.");
    println!("{:#?}", pipeline);
    println!("Tracing is enabled. This console will not show any logging information.");

    let tracer = pipeline
        .install_batch(opentelemetry::runtime::Tokio)
        .unwrap();

    tracing_subscriber::Registry::default()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .init();
}

fn init_no_tracing() {
    env_logger::builder().format_timestamp_millis().init();
    log::info!("No tracing subscriber is active, logging stays active");
}
