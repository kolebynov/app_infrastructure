use std::{error::Error, path::{PathBuf, Path}, io};

use config::{Config, ConfigError};
use tracing::Level;
use tracing_appender::rolling::RollingFileAppender;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, filter::LevelFilter, Layer, Registry, fmt::{format, FormatFields, FormatEvent, MakeWriter}};
use serde::{Deserialize, de::{Visitor, Unexpected}};

use crate::BoxError;

type FmtLayer<S, N = format::DefaultFields, E = format::Format<format::Full>, W = fn() -> io::Stdout> =
    tracing_subscriber::fmt::Layer<S, N, E, W>;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LevelWrapper(Level);

impl Default for LevelWrapper {
    fn default() -> Self {
        Self(Level::INFO)
    }
}

impl<'de> Deserialize<'de> for LevelWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>
    {
        struct LevelWrapperVisitor {}

        impl<'de> Visitor<'de> for LevelWrapperVisitor {
            type Value = LevelWrapper;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "This Visitor expects to receive string")
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
                    _ => return Err(serde::de::Error::invalid_value(Unexpected::Str(value), &"Correct log level")),
                };

                Ok(LevelWrapper(level))
            }
        }

        deserializer.deserialize_str(LevelWrapperVisitor {})
    }
}

trait LayerConfigurator
{
    type Fields: for<'writer> FormatFields<'writer> + Send + Sync + 'static;
    type Event: FormatEvent<Registry, Self::Fields> + Send + Sync + 'static;
    type Writer: for<'writer> MakeWriter<'writer> + Send + Sync + 'static;

    fn configure(&self, layer: FmtLayer<Registry>) -> Result<FmtLayer<Registry, Self::Fields, Self::Event, Self::Writer>, BoxError>;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StdoutWriterConfig {}

impl LayerConfigurator for StdoutWriterConfig
{
    type Fields = format::DefaultFields;
    type Event = format::Format<format::Full>;
    type Writer = fn() -> io::Stdout;

    fn configure(&self, layer: FmtLayer<Registry>) -> Result<FmtLayer<Registry, Self::Fields, Self::Event, Self::Writer>, BoxError> {
        Ok(layer.with_ansi(true))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollingFileWriterConfig {
    pub log_path: PathBuf,
}

impl LayerConfigurator for RollingFileWriterConfig
{
    type Fields = format::DefaultFields;
    type Event = format::Format<format::Full>;
    type Writer = RollingFileAppender;

    fn configure(&self, layer: FmtLayer<Registry>) -> Result<FmtLayer<Registry, Self::Fields, Self::Event, Self::Writer>, BoxError> {
        let dir_path = self.log_path.parent().unwrap_or(Path::new(""));
        let file_appender = tracing_appender::rolling::hourly(
            dir_path,
            self.log_path.file_name().ok_or("Invalid log path".to_string())?.to_string_lossy().to_string());
        Ok(layer.with_ansi(false).with_writer(file_appender))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WriterConfig {
    Stdout(StdoutWriterConfig),
    RollingFile(RollingFileWriterConfig),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerConfig {
    pub level: Option<LevelWrapper>,
    pub writer: WriterConfig,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TracingConfig {
    pub default_level: LevelWrapper,
    pub layers: Option<Vec<LayerConfig>>,
}

pub fn init_from_config(config: &Config) -> Result<(), BoxError> {
    let logging_config = config.get::<TracingConfig>("tracing")
        .or_else(|err| if matches!(err, ConfigError::NotFound(_)) { Ok(TracingConfig::default()) } else { Err(err) })?;
    let default_level = logging_config.default_level;

    let mut layers = vec![];
    for layer_config in logging_config.layers.unwrap_or(vec![]) {
        let level_filter = LevelFilter::from_level(layer_config.level.unwrap_or(default_level).0);
        let layer = match layer_config.writer {
            WriterConfig::Stdout(stdout_config) => configure_layer(stdout_config, level_filter),
            WriterConfig::RollingFile(rolling_config) => configure_layer(rolling_config, level_filter),
        };

        layers.push(layer?);
    }

    tracing_subscriber::registry()
        .with(layers)
        .try_init()?;

    Ok(())
}

fn configure_layer(configurator: impl LayerConfigurator, level_filter: LevelFilter) -> Result<Box<dyn Layer<Registry> + Send + Sync>, BoxError> {
    Ok(configurator.configure(FmtLayer::default())?
        .with_filter(level_filter)
        .boxed())
}