/// Configuration support
#[cfg(all(feature = "config", feature = "serde", feature = "std"))]
pub mod config;
/// Working with [`Default`]
pub mod default;
/// Application information
pub mod info;
/// Spawning tasks
pub mod spawn;
#[cfg(all(feature = "std", feature = "serde"))]
pub mod tls;

pub use spawn::Spawner;
