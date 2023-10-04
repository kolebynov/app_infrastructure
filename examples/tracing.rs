use app_infrastructure::{app_config::AppConfigurationBuilder, app_tracing, BoxError};
use config::{Config, Environment, File};
use tracing::{debug, error, info, trace, warn};

fn main() -> Result<(), BoxError> {
    let app_config = AppConfigurationBuilder::new().build_with_custom_config_builder(|info| {
        Config::builder()
            .add_source(File::with_name("examples/app_settings"))
            .add_source(
                Environment::with_prefix(&info.env_prefix)
                    .try_parsing(true)
                    .separator("."),
            )
    })?;
    app_tracing::init_from_config(&app_config.config)?;

    trace!("Trace");
    debug!("Debug");
    info!("Info");
    warn!("Warn");
    error!("Error");

    Ok(())
}
