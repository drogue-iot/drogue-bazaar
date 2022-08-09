#[cfg(feature = "actix")]
mod actix;

#[cfg(feature = "actix")]
pub use actix::HealthServer;

use crate::health::{HealthCheckError, HealthChecked};
use futures_util::stream::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use std::future::Future;
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::sync::RwLock;
use tracing::instrument;

#[derive(Clone, Debug, Deserialize)]
pub struct HealthServerConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "defaults::bind_addr")]
    pub bind_addr: String,
    #[serde(default = "defaults::workers")]
    pub workers: usize,
}

mod defaults {
    #[inline]
    pub fn bind_addr() -> String {
        "[::1]:9090".into()
    }

    #[inline]
    pub fn workers() -> usize {
        1
    }
}

impl Default for HealthServerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bind_addr: defaults::bind_addr(),
            workers: defaults::workers(),
        }
    }
}

/// Internal handling of health checking.
#[derive(Clone, Default)]
pub struct HealthChecker {
    checks: Arc<RwLock<Vec<Box<dyn HealthChecked>>>>,
}

impl HealthChecker {
    #[instrument(level = "trace", skip(self), ret)]
    pub async fn is_ready(&self) -> Vec<Result<(), HealthCheckError>> {
        futures_util::stream::iter(self.checks.read().await.iter())
            .then(|check| check.is_ready())
            .collect()
            .await
    }

    #[instrument(level = "trace", skip(self), ret)]
    pub async fn is_alive(&self) -> Vec<Result<(), HealthCheckError>> {
        futures_util::stream::iter(self.checks.read().await.iter())
            .then(|check| check.is_alive())
            .collect()
            .await
    }

    pub fn push<C>(&self, check: C)
    where
        C: Into<Box<dyn HealthChecked + 'static>>,
    {
        let check = check.into();
        let checks = self.checks.clone();
        Handle::current().spawn(async move {
            checks.write().await.push(check);
        });
    }
}

impl<C> Extend<C> for HealthChecker
where
    C: HealthChecked + 'static,
{
    fn extend<T: IntoIterator<Item = C>>(&mut self, iter: T) {
        // collect first, so that we can send/spawn it
        let iter: Vec<_> = iter
            .into_iter()
            .map(|c| Box::new(c) as Box<dyn HealthChecked>)
            .collect();
        let checks = self.checks.clone();
        Handle::current().spawn(async move {
            checks.write().await.extend(iter);
        });
    }
}

impl Extend<Box<dyn HealthChecked>> for HealthChecker {
    fn extend<T: IntoIterator<Item = Box<dyn HealthChecked + 'static>>>(&mut self, iter: T) {
        // collect first, so that we can send/spawn it
        let iter: Vec<_> = iter.into_iter().collect();
        let checks = self.checks.clone();
        Handle::current().spawn(async move {
            checks.write().await.extend(iter);
        });
    }
}

#[allow(unused)]
async fn run_checks<F, Fut>(checker: Arc<HealthChecker>, f: F) -> (http::StatusCode, Value)
where
    F: FnOnce(Arc<HealthChecker>) -> Fut,
    Fut: Future<Output = Vec<Result<(), HealthCheckError>>>,
{
    let result: Result<Vec<()>, _> = f(checker).await.into_iter().collect();

    match result {
        Ok(_) => (http::StatusCode::OK, json!({ "success": true})),
        Err(_) => (
            http::StatusCode::SERVICE_UNAVAILABLE,
            json!({"success": false}),
        ),
    }
}
