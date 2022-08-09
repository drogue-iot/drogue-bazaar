mod run;

pub use run::{HealthChecker, HealthServerConfig};

#[cfg(feature = "actix")]
pub use run::HealthServer;
