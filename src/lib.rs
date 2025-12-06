pub mod app_config;
#[cfg(feature = "app_tracing")]
pub mod app_tracing;

#[cfg(feature = "app_tracing")]
pub use tracing;

pub use config;

use std::error::Error;

pub type BoxError = Box<dyn Error + Send + Sync>;
