mod run;

pub use run::{HealthServer, HealthServerConfig};

use async_trait::async_trait;
use core::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum HealthCheckError {
    Failed(Box<dyn std::error::Error>),
    NotOk(String),
}

impl std::error::Error for HealthCheckError {}

impl Display for HealthCheckError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed(err) => write!(f, "Health check failed: {err}"),
            Self::NotOk(reason) => write!(f, "Not OK: {reason}"),
        }
    }
}

impl<E> From<Box<E>> for HealthCheckError
where
    E: std::error::Error + 'static,
{
    fn from(err: Box<E>) -> Self {
        Self::Failed(err)
    }
}

impl HealthCheckError {
    pub fn nok<T, S: Into<String>>(reason: S) -> Result<T, Self> {
        Err(Self::NotOk(reason.into()))
    }
}

#[async_trait]
pub trait HealthChecked: Send + Sync {
    async fn is_ready(&self) -> Result<(), HealthCheckError> {
        Ok(())
    }

    async fn is_alive(&self) -> Result<(), HealthCheckError> {
        Ok(())
    }
}

pub trait BoxedHealthChecked {
    fn boxed(self) -> Box<dyn HealthChecked>;
}

impl<T> BoxedHealthChecked for T
where
    T: HealthChecked + 'static,
{
    fn boxed(self) -> Box<dyn HealthChecked> {
        Box::new(self)
    }
}
