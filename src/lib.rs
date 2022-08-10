//! A place to find tools for building your Rust application.

#[cfg(all(
    feature = "actix",
    any(
        feature = "default-tls",
        feature = "native-tls",
        feature = "rustls-tls"
    )
))]
pub mod actix;
#[cfg(feature = "app")]
pub mod app;
pub mod auth;
#[cfg(any(
    feature = "default-tls",
    feature = "native-tls",
    feature = "rustls-tls"
))]
pub mod client;
pub mod core;
pub mod health;
#[cfg(any(
    feature = "default-tls",
    feature = "native-tls",
    feature = "rustls-tls"
))]
pub mod reqwest;

#[doc(hidden)]
pub mod prelude {
    pub use crate::core::default::is_default;
}
