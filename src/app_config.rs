use std::{env, fmt::Display};

use config::{Config, ConfigBuilder, ConfigError, Environment, File, builder::DefaultState};

const APP_ENVIRONMENT_KEY: &str = "ENVIRONMENT";
const DEFAULT_ENVIRONMENT: AppEnvironment = AppEnvironment::Dev;
const DEFAULT_ENV_PREFIX: &str = "RUST_APP";
const DEFAULT_ENV_SEPARATOR: &str = "__";

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

pub struct ConfigBuildingInfo {
    pub app_environment: AppEnvironment,
    pub env_prefix: String,
    pub env_separator: String,
}

pub struct AppConfigurationBuilder {
    env_prefix: String,
    env_separator: String,
}

impl AppConfigurationBuilder {
    pub fn new() -> Self {
        Self {
            env_prefix: DEFAULT_ENV_PREFIX.to_string(),
            env_separator: DEFAULT_ENV_SEPARATOR.to_string(),
        }
    }

    pub fn with_custom_env_prefix(self, env_prefix: String) -> Self {
        Self { env_prefix, ..self }
    }

    pub fn with_custom_env_separator(self, env_separator: String) -> Self {
        Self {
            env_separator,
            ..self
        }
    }

    pub fn build_with_custom_config_builder(
        self,
        configurator: impl FnOnce(ConfigBuildingInfo) -> ConfigBuilder<DefaultState>,
    ) -> Result<AppConfiguration, ConfigError> {
        let app_environment = get_app_environment(&self.env_prefix);
        let config = configurator(ConfigBuildingInfo {
            app_environment: app_environment.clone(),
            env_prefix: self.env_prefix,
            env_separator: self.env_separator,
        })
        .build()?;

        Ok(AppConfiguration {
            app_environment,
            config,
        })
    }

    pub fn build(self) -> Result<AppConfiguration, ConfigError> {
        self.build_with_custom_config_builder(|info| {
            Config::builder()
                .add_source(File::with_name("app_settings").required(false))
                .add_source(
                    File::with_name(&format!("app_settings.{}", info.app_environment))
                        .required(false),
                )
                .add_source(
                    Environment::with_prefix(&info.env_prefix)
                        .try_parsing(true)
                        .separator(&info.env_separator),
                )
        })
    }
}

impl Default for AppConfigurationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn get_app_environment(prefix: &str) -> AppEnvironment {
    env::var(format!("{prefix}{}", APP_ENVIRONMENT_KEY))
        .map(|s| s.as_str().into())
        .unwrap_or(DEFAULT_ENVIRONMENT)
}

pub struct AppConfiguration {
    pub app_environment: AppEnvironment,
    pub config: Config,
}
