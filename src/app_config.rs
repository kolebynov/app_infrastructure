use std::{fmt::Display, env};

use config::{Config, ConfigError, Environment, File, ConfigBuilder, builder::DefaultState};

const APP_ENVIRONMENT_KEY: &str = "RUST_APP_ENVIRONMENT";
const DEFAULT_ENVIRONMENT: AppEnvironment = AppEnvironment::Dev;

#[derive(Clone)]
pub enum AppEnvironment {
    Dev,
    Prod,
    Custom(String),
}

impl Display for AppEnvironment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppEnvironment::Dev => write!(f, "dev"),
            AppEnvironment::Prod => write!(f, "prod"),
            AppEnvironment::Custom(app_env) => write!(f, "{}", app_env),
        }
    }
}

impl From<&str> for AppEnvironment {
    fn from(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "dev" => AppEnvironment::Dev,
            "prod" => AppEnvironment::Prod,
            _ => AppEnvironment::Custom(value.to_string()),
        }
    }
}

pub struct AppConfigurationBuilder {
    config_builder: ConfigBuilder<DefaultState>,
    app_environment: AppEnvironment,
}

impl AppConfigurationBuilder {
    pub fn new() -> Self {
        Self {
            config_builder: Config::builder(),
            app_environment: get_app_environment(),
        }
    }

    pub fn configure_config_builder(self, configurator: impl FnOnce(ConfigBuilder<DefaultState>, AppEnvironment) -> ConfigBuilder<DefaultState>) -> Self {
        Self {
            config_builder: configurator(self.config_builder, self.app_environment.clone()),
            ..self
        }
    }

    pub fn build(self) -> Result<AppConfiguration, ConfigError> {
        Ok(AppConfiguration {
            app_environment: self.app_environment,
            config: self.config_builder.build()?,
        })
    }
}

impl Default for AppConfigurationBuilder {
    fn default() -> Self {
        AppConfigurationBuilder::new()
            .configure_config_builder(|builder, app_env| {
                builder
                    .add_source(File::with_name("app_settings").required(false))
                    .add_source(File::with_name(&format!("app_settings.{}", app_env)).required(false))
                    .add_source(Environment::default())
            })
    }
}

fn get_app_environment() -> AppEnvironment {
    env::var(APP_ENVIRONMENT_KEY).map(|s| s.as_str().into()).unwrap_or(DEFAULT_ENVIRONMENT)
}

pub struct AppConfiguration {
    pub app_environment: AppEnvironment,
    pub config: Config,
}