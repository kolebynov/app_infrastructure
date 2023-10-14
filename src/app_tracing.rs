use std::{
    io,
    path::{Path, PathBuf},
};

use config::{Config, ConfigError};
use serde::{
    de::{Unexpected, Visitor},
    Deserialize,
};
use tracing_appender::rolling::RollingFileAppender;
use tracing_subscriber::{
    filter::LevelFilter,
    fmt::{format, FormatEvent, FormatFields, MakeWriter},
    prelude::__tracing_subscriber_SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer, Registry,
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

pub struct EnvFilterWrapper(pub EnvFilter, pub String);

impl<'de> Deserialize<'de> for EnvFilterWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct EnvFilterWrapperVisitor;

        impl<'a> Visitor<'a> for EnvFilterWrapperVisitor {
            type Value = EnvFilterWrapper;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(
                    formatter,
                    "an array of values in EnvFilter format \"target[span{{field=value}}]=level\""
                )
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'a>,
            {
                let mut filter_str = String::new();
                while let Some(str) = seq.next_element::<String>()? {
                    filter_str.push_str(&str);
                    filter_str.push(',');
                }

                if !filter_str.is_empty() {
                    filter_str.remove(filter_str.len() - 1);
                }

                let filter = EnvFilter::try_new(&filter_str).map_err(|err| {
                    serde::de::Error::invalid_value(
                        Unexpected::Str(&filter_str),
                        &err.to_string().as_ref(),
                    )
                })?;

                Ok(EnvFilterWrapper(filter, filter_str))
            }
        }

        deserializer.deserialize_seq(EnvFilterWrapperVisitor)
    }
}

impl Default for EnvFilterWrapper {
    fn default() -> Self {
        Self(
            EnvFilter::new("").add_directive(LevelFilter::INFO.into()),
            String::default(),
        )
    }
}

impl Clone for EnvFilterWrapper {
    fn clone(&self) -> Self {
        Self(EnvFilter::new(&self.1), self.1.clone())
    }
}

#[derive(Deserialize)]
pub struct LayerConfig {
    #[serde(default)]
    pub filter: Option<EnvFilterWrapper>,
    pub writer: WriterConfig,
}

#[derive(Default, Deserialize)]
pub struct TracingConfig {
    pub filter: EnvFilterWrapper,
    pub layers: Option<Vec<LayerConfig>>,
}

pub fn init_from_config(config: &Config) -> Result<(), BoxError> {
    let logging_config = config.get::<TracingConfig>("tracing").or_else(|err| {
        if matches!(err, ConfigError::NotFound(_)) {
            Ok(TracingConfig::default())
        } else {
            Err(err)
        }
    })?;

    let default_level = logging_config.filter;

    let mut layers = vec![];
    for layer_config in logging_config.layers.unwrap_or(vec![]) {
        let filter = layer_config
            .filter
            .unwrap_or_else(|| default_level.clone())
            .0;
        let layer = match layer_config.writer {
            WriterConfig::Stdout(stdout_config) => configure_layer(stdout_config, filter),
            WriterConfig::RollingFile(rolling_config) => configure_layer(rolling_config, filter),
        };

        layers.push(layer?);
    }

    tracing_subscriber::registry().with(layers).try_init()?;

    Ok(())
}

fn configure_layer(
    configurator: impl LayerConfigurator,
    filter: EnvFilter,
) -> Result<Box<dyn Layer<Registry> + Send + Sync>, BoxError> {
    Ok(configurator
        .configure(FmtLayer::default())?
        .with_filter(filter)
        .boxed())
}
