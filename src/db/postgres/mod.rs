//! Basic PostgreSQL support

use crate::core::tls::ClientConfig;

/// A Postgres pooled connection configuration
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub db: deadpool_postgres::Config,
    #[serde(default)]
    pub tls: ClientConfig,
}

impl Config {
    /// Create a pool from a configuration.
    pub fn create_pool(&self) -> anyhow::Result<deadpool_postgres::Pool> {
        Ok(self.db.create_pool(
            Some(deadpool_postgres::Runtime::Tokio1),
            postgres_native_tls::MakeTlsConnector::new((&self.tls).try_into()?),
        )?)
    }
}
