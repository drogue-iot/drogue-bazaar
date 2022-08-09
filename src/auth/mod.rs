//! Authentication and authorization tooling.

pub mod authz;
#[cfg(any(
    feature = "default-tls",
    feature = "native-tls",
    feature = "rustls-tls"
))]
pub mod openid;
pub mod pat;

mod error;
mod user;
pub use error::*;
pub use user::*;
