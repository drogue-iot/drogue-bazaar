/// TLS client authentication mode.
#[derive(Clone, Copy)]
pub enum TlsMode {
    /// No client authentication
    NoClient,
    /// with Drogue specific client authentication
    Client,
}

pub trait TlsPskHandler:
    Fn(Option<&[u8]>, &mut [u8]) -> Result<usize, std::io::Error> + Sync + Send
{
}

/// TLS configuration
pub struct TlsAuthConfig {
    pub mode: TlsMode,
    pub psk: Option<Box<dyn TlsPskHandler>>,
}

impl Default for TlsAuthConfig {
    fn default() -> Self {
        Self {
            mode: TlsMode::NoClient,
            psk: None,
        }
    }
}

/// Syntactic sugar for working with [`TlsAuthConfig`].
pub trait WithTlsAuthConfig {
    fn with_tls_auth_config(&self, tls_config: TlsAuthConfig) -> Option<TlsAuthConfig>;
}

/// Boolean flag means disable.
impl WithTlsAuthConfig for bool {
    fn with_tls_auth_config(&self, tls_config: TlsAuthConfig) -> Option<TlsAuthConfig> {
        if *self {
            None
        } else {
            Some(tls_config)
        }
    }
}
