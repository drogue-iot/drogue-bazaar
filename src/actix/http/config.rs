use super::defaults;
use crate::actix::http::CorsConfig;
use serde::Deserialize;

/// HTTP server configuration.
#[derive(Clone, Debug, Deserialize)]
pub struct HttpConfig {
    #[serde(default = "defaults::bind_addr")]
    pub bind_addr: String,
    #[serde(default = "defaults::max_json_payload_size")]
    pub max_json_payload_size: usize,
    #[serde(default = "defaults::max_payload_size")]
    pub max_payload_size: usize,
    #[serde(default)]
    pub disable_tls: bool,
    #[serde(default)]
    pub disable_tls_psk: bool,
    #[serde(default)]
    pub cert_bundle_file: Option<String>,
    #[serde(default)]
    pub key_file: Option<String>,

    #[serde(default)]
    pub workers: Option<usize>,

    #[serde(default)]
    pub metrics_namespace: Option<String>,

    #[serde(default)]
    pub cors: Option<CorsConfig>,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            bind_addr: defaults::bind_addr(),
            max_json_payload_size: defaults::max_json_payload_size(),
            max_payload_size: defaults::max_payload_size(),
            disable_tls: false,
            disable_tls_psk: false,
            cert_bundle_file: None,
            key_file: None,
            workers: None,
            metrics_namespace: None,
            cors: None,
        }
    }
}
