/// TLS client authentication mode.
#[derive(Clone, Copy)]
pub enum TlsMode {
    /// No client authentication
    NoClient,
    /// with Drogue specific client authentication
    Client,
}

/// Syntactic sugar for working with [`TlsMode`].
pub trait WithTlsMode {
    fn with_tls_mode(&self, tls_mode: TlsMode) -> Option<TlsMode>;
}

/// Boolean flag means disable.
impl WithTlsMode for bool {
    fn with_tls_mode(&self, tls_mode: TlsMode) -> Option<TlsMode> {
        if *self {
            None
        } else {
            Some(tls_mode)
        }
    }
}
