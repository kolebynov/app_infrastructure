pub mod app_config;
#[cfg(feature = "app_tracing")]
pub mod app_tracing;
#[cfg(feature = "tonic")]
pub mod tonic;

pub use config;

use std::error::Error;

pub type BoxError = Box<dyn Error + Send + Sync>;