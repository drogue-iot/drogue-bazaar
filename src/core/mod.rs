//! Core functionality.

/// Configuration support
pub mod config;
/// Working with [`Default`]
pub mod default;
/// Application information
pub mod info;
/// Spawning tasks
pub mod spawn;
pub mod tls;

pub use spawn::{Spawner, SpawnerExt};
