//! Authentication and authorization tooling.

#[cfg(not(target_arch = "wasm32"))]
pub mod authz;
#[cfg(any(
    feature = "default-tls",
    feature = "native-tls",
    feature = "rustls-tls"
))]
#[cfg(not(target_arch = "wasm32"))]
pub mod openid;
#[cfg(not(target_arch = "wasm32"))]
pub mod pat;

mod error;
mod user;
pub use error::*;
pub use user::*;
