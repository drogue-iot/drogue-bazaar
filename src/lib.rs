//! A place to find tools for building your Rust application.

#[cfg(feature = "actix")]
pub use drogue_bazaar_actix as actix;
#[cfg(feature = "app")]
pub use drogue_bazaar_application as app;
pub use drogue_bazaar_core as core;

#[cfg(feature = "app")]
pub mod macros;

pub use drogue_bazaar_core::{component, project};

pub mod prelude {
    pub use crate::core::default::is_default;
}
