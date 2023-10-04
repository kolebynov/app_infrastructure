use std::{
    io,
    path::{Path, PathBuf},
};

use config::{Config, ConfigError};
use serde::{
    de::{Unexpected, Visitor},
    Deserialize, Deserializer,
};
use tracing::Level;
use tracing_appender::rolling::RollingFileAppender;
use tracing_subscriber::{
    filter::LevelFilter,
    fmt::{format, FormatEvent, FormatFields, MakeWriter},
    prelude::__tracing_subscriber_SubscriberExt,
    util::SubscriberInitExt,
    Layer, Registry,
};

use crate::BoxError;

type FmtLayer<
    S,
    N = format::DefaultFields,
    E = format::Format<format::Full>,
    W = fn() -> io::Stdout,
> = tracing_subscriber::fmt::Layer<S, N, E, W>;

trait LayerConfigurator {
    type Fields: for<'writer> FormatFields<'writer> + Send + Sync + 'static;
    type Event: FormatEvent<Registry, Self::Fields> + Send + Sync + 'static;
    type Writer: for<'writer> MakeWriter<'writer> + Send + Sync + 'static;

    fn configure(
        &self,
        layer: FmtLayer<Registry>,
    ) -> Result<FmtLayer<Registry, Self::Fields, Self::Event, Self::Writer>, BoxError>;
}

#[derive(Deserialize)]
pub struct StdoutWriterConfig {}

impl LayerConfigurator for StdoutWriterConfig {
    type Fields = format::DefaultFields;
    type Event = format::Format<format::Full>;
    type Writer = fn() -> io::Stdout;

    fn configure(
        &self,
        layer: FmtLayer<Registry>,
    ) -> Result<FmtLayer<Registry, Self::Fields, Self::Event, Self::Writer>, BoxError> {
        Ok(layer.with_ansi(true))
    }
}

#[derive(Deserialize)]
pub struct RollingFileWriterConfig {
    pub log_path: PathBuf,
}

impl LayerConfigurator for RollingFileWriterConfig {
    type Fields = format::DefaultFields;
    type Event = format::Format<format::Full>;
    type Writer = RollingFileAppender;

    fn configure(
        &self,
        layer: FmtLayer<Registry>,
    ) -> Result<FmtLayer<Registry, Self::Fields, Self::Event, Self::Writer>, BoxError> {
        let dir_path = self.log_path.parent().unwrap_or(Path::new(""));
        let file_appender = tracing_appender::rolling::hourly(
            dir_path,
            self.log_path
                .file_name()
                .ok_or("Invalid log path".to_string())?
                .to_string_lossy()
                .to_string(),
        );
        Ok(layer.with_ansi(false).with_writer(file_appender))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriterConfig {
    Stdout(StdoutWriterConfig),
    RollingFile(RollingFileWriterConfig),
}

#[derive(Deserialize)]
pub struct LayerConfig {
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_level_option")]
    pub level: Option<Level>,
    pub writer: WriterConfig,
}

#[derive(Deserialize)]
pub struct TracingConfig {
    #[serde(deserialize_with = "deserialize_level")]
    pub default_level: Level,
    pub layers: Option<Vec<LayerConfig>>,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            default_level: Level::INFO,
            layers: Default::default(),
        }
    }
}

pub fn init_from_config(config: &Config) -> Result<(), BoxError> {
    let logging_config = config.get::<TracingConfig>("tracing").or_else(|err| {
        if matches!(err, ConfigError::NotFound(_)) {
            Ok(TracingConfig::default())
        } else {
            Err(err)
        }
    })?;

    let default_level = logging_config.default_level;

    let mut layers = vec![];
    for layer_config in logging_config.layers.unwrap_or(vec![]) {
        let level_filter = LevelFilter::from_level(layer_config.level.unwrap_or(default_level));
        let layer = match layer_config.writer {
            WriterConfig::Stdout(stdout_config) => configure_layer(stdout_config, level_filter),
            WriterConfig::RollingFile(rolling_config) => {
                configure_layer(rolling_config, level_filter)
            }
        };

        layers.push(layer?);
    }

    tracing_subscriber::registry().with(layers).try_init()?;

    Ok(())
}

fn configure_layer(
    configurator: impl LayerConfigurator,
    level_filter: LevelFilter,
) -> Result<Box<dyn Layer<Registry> + Send + Sync>, BoxError> {
    Ok(configurator
        .configure(FmtLayer::default())?
        .with_filter(level_filter)
        .boxed())
}

fn deserialize_level<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Level, D::Error> {
    Ok(deserialize_level_option(deserializer)?.unwrap())
}

fn deserialize_level_option<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Level>, D::Error> {
    struct LevelVisitor {}

    impl<'de> Visitor<'de> for LevelVisitor {
        type Value = Option<Level>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "This Visitor expects to receive string")
        }

        fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let level = match value {
                "Trace" => Level::TRACE,
                "Debug" => Level::DEBUG,
                "Info" => Level::INFO,
                "Warn" => Level::WARN,
                "Error" => Level::ERROR,
                _ => {
                    return Err(serde::de::Error::invalid_value(
                        Unexpected::Str(value),
                        &"Correct log level",
                    ))
                }
            };

            Ok(Some(level))
        }
    }

    deserializer.deserialize_str(LevelVisitor {})
}
