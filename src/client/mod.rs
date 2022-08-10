//! Working with API clients.

use crate::{auth::openid::TokenConfig, reqwest::ClientFactory};
use async_trait::async_trait;
use drogue_client::openid::TokenProvider;
use url::Url;

/// A standard API client configuration.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct ClientConfig {
    pub url: Url,

    #[serde(flatten, default)]
    pub token_config: Option<TokenConfig>,
}

impl ClientConfig {
    /// Convert into a client.
    pub async fn into_client<T>(self) -> anyhow::Result<T>
    where
        T: ClientCreator,
    {
        let token = if let Some(token) = self.token_config {
            Some(token.discover_from().await?)
        } else {
            None
        };

        T::new(ClientFactory::new().build()?, self.url, token)
    }
}

/// Create a new client from a URL, an HTTP client, and a token provider.
#[async_trait]
pub trait ClientCreator: Sized {
    fn new<TP>(client: reqwest::Client, url: Url, token_provider: TP) -> anyhow::Result<Self>
    where
        TP: TokenProvider + 'static;
}

impl ClientCreator for drogue_client::user::v1::Client {
    fn new<TP>(client: reqwest::Client, url: Url, token_provider: TP) -> anyhow::Result<Self>
    where
        TP: TokenProvider + 'static,
    {
        let authn_url = url.join("/api/user/v1alpha1/authn")?;
        let authz_url = url.join("/api/v1/user/authz")?;

        Ok(Self::new(client, authn_url, authz_url, token_provider))
    }
}

impl ClientCreator for drogue_client::registry::v1::Client {
    fn new<TP>(client: reqwest::Client, url: Url, token_provider: TP) -> anyhow::Result<Self>
    where
        TP: TokenProvider + 'static,
    {
        Ok(Self::new(client, url, token_provider))
    }
}

impl ClientCreator for drogue_client::command::v1::Client {
    fn new<TP>(client: reqwest::Client, url: Url, token_provider: TP) -> anyhow::Result<Self>
    where
        TP: TokenProvider + 'static,
    {
        Ok(Self::new(client, url, token_provider))
    }
}

impl ClientCreator for drogue_client::admin::v1::Client {
    fn new<TP>(client: reqwest::Client, url: Url, token_provider: TP) -> anyhow::Result<Self>
    where
        TP: TokenProvider + 'static,
    {
        Ok(Self::new(client, url, token_provider))
    }
}
