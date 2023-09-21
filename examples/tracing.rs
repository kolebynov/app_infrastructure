use std::env;

use app_infrastructure::{BoxError, app_config::AppConfigurationBuilder, app_tracing};
use config::{File, Environment};
use tracing::{trace, debug, info, warn, error};

fn main() -> Result<(), BoxError> {
    let app_config = AppConfigurationBuilder::new()
        .configure_config_builder(|builder, _| builder
            .add_source(File::with_name("examples/app_settings"))
            .add_source(Environment::default()))
        .build()?;
    app_tracing::init_from_config(&app_config.config)?;

    trace!("Trace");
    debug!("Debug");
    info!("Info");
    warn!("Warn");
    error!("Error");

    Ok(())
}