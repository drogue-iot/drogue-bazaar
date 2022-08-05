//! A place to find tools for building your Rust application.

pub mod actix;
pub mod app;
pub mod core;
pub mod health;

pub mod prelude {
    pub use crate::core::default::is_default;
}
