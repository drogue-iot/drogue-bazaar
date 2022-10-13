/// TLS client authentication mode.
#[derive(Clone, Copy)]
pub enum TlsMode {
    /// No client authentication
    NoClient,
    /// with Drogue specific client authentication
    Client,
}
