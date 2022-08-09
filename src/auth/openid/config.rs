use crate::{core::config::CommaSeparatedVec, reqwest::ClientFactory};
use anyhow::Context;
use core::fmt::Debug;
use drogue_client::openid::OpenIdTokenProvider;
use serde::Deserialize;
use std::time::Duration;
use url::Url;

/// All required configuration when authentication is enabled.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct TokenConfig {
    pub client_id: String,

    pub client_secret: String,

    pub issuer_url: Url,

    #[serde(default)]
    pub tls_insecure: bool,

    #[serde(default)]
    pub tls_ca_certificates: CommaSeparatedVec,

    #[serde(default)]
    #[serde(with = "humantime_serde")]
    pub refresh_before: Option<Duration>,
}

impl TokenConfig {
    pub async fn into_client(self, redirect: Option<String>) -> anyhow::Result<openid::Client> {
        let mut client = ClientFactory::new();
        client = client.add_ca_certs(self.tls_ca_certificates.0);

        if self.tls_insecure {
            client = client.make_insecure();
        }

        openid::Client::discover_with_client(
            client.build()?,
            self.client_id,
            self.client_secret,
            redirect,
            self.issuer_url,
        )
        .await
        .context("Discovering endpoint")
    }

    /// Create a new provider by discovering the OAuth2 client from the configuration
    pub async fn discover_from(self) -> anyhow::Result<OpenIdTokenProvider> {
        let refresh_before = self
            .refresh_before
            .and_then(|d| chrono::Duration::from_std(d).ok())
            .unwrap_or_else(|| chrono::Duration::seconds(15));

        Ok(OpenIdTokenProvider::new(
            self.into_client(None).await?,
            refresh_before,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::core::config::ConfigFromEnv;
    use std::collections::HashMap;

    #[test]
    fn test_ca_certs() {
        let mut envs = HashMap::new();

        envs.insert("CLIENT_ID", "id");
        envs.insert("CLIENT_SECRET", "secret");
        envs.insert("ISSUER_URL", "http://foo.bar/baz/buz");
        envs.insert("REALM", "drogue");
        envs.insert("TLS_CA_CERTIFICATES", "/foo/bar/baz");

        let config = TokenConfig::from_set(envs).unwrap();

        assert_eq!(
            TokenConfig {
                client_id: "id".to_string(),
                client_secret: "secret".to_string(),
                issuer_url: Url::parse("http://foo.bar/baz/buz").unwrap(),
                refresh_before: None,
                tls_insecure: false,
                tls_ca_certificates: vec!["/foo/bar/baz".to_string()].into(),
            },
            config
        );
    }

    #[test]
    fn test_ca_certs_multi() {
        let mut envs = HashMap::new();

        envs.insert("CLIENT_ID", "id");
        envs.insert("CLIENT_SECRET", "secret");
        envs.insert("ISSUER_URL", "http://foo.bar/baz/buz");
        envs.insert("REALM", "drogue");
        envs.insert("TLS_CA_CERTIFICATES", "/foo/bar/baz,/foo/bar/baz2");

        let config = TokenConfig::from_set(envs).unwrap();

        assert_eq!(
            TokenConfig {
                client_id: "id".to_string(),
                client_secret: "secret".to_string(),
                issuer_url: Url::parse("http://foo.bar/baz/buz").unwrap(),
                refresh_before: None,
                tls_insecure: false,
                tls_ca_certificates: vec!["/foo/bar/baz".to_string(), "/foo/bar/baz2".to_string()]
                    .into(),
            },
            config
        );
    }
}
