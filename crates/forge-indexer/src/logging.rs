//! Comprehensive logging and observability configuration

use std::env;

use anyhow::Result;
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::{self};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, Registry};

/// Logging configuration for forge-indexer
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: Level,
    /// Whether to include file and line numbers
    pub include_location: bool,
    /// Whether to include thread IDs
    pub include_thread_ids: bool,
    /// Whether to include timestamps
    pub include_timestamps: bool,
    /// Whether to use JSON formatting
    pub json_format: bool,
    /// Whether to log spans (enter/exit)
    pub log_spans: bool,
    /// Environment filter string
    pub env_filter: Option<String>,
    /// Whether to enable performance metrics
    pub enable_metrics: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: Level::INFO,
            include_location: true,
            include_thread_ids: true,
            include_timestamps: true,
            json_format: false,
            log_spans: false,
            env_filter: None,
            enable_metrics: true,
        }
    }
}

impl LoggingConfig {
    /// Create a new logging configuration from environment variables
    pub fn from_env() -> Self {
        let level = env::var("LOG_LEVEL")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(Level::INFO);

        let include_location = env::var("LOG_INCLUDE_LOCATION")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);

        let include_thread_ids = env::var("LOG_INCLUDE_THREAD_IDS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);

        let include_timestamps = env::var("LOG_INCLUDE_TIMESTAMPS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);

        let json_format = env::var("LOG_JSON_FORMAT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(false);

        let log_spans = env::var("LOG_SPANS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(false);

        let env_filter = env::var("RUST_LOG").ok();

        let enable_metrics = env::var("ENABLE_METRICS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);

        Self {
            level,
            include_location,
            include_thread_ids,
            include_timestamps,
            json_format,
            log_spans,
            env_filter,
            enable_metrics,
        }
    }

    /// Initialize the global tracing subscriber
    pub fn init_tracing(&self) -> Result<()> {
        let env_filter = self.build_env_filter()?;

        if self.json_format {
            self.init_json_logging(env_filter)?;
        } else {
            self.init_pretty_logging(env_filter)?;
        }

        Ok(())
    }

    fn build_env_filter(&self) -> Result<EnvFilter> {
        let filter = if let Some(ref env_filter) = self.env_filter {
            EnvFilter::try_new(env_filter)?
        } else {
            EnvFilter::default()
                .add_directive(format!("forge_indexer={}", self.level).parse()?)
                .add_directive("tower_http=debug".parse()?)
                .add_directive("axum=debug".parse()?)
                .add_directive("hyper=info".parse()?)
                .add_directive("tokio=info".parse()?)
                .add_directive("qdrant_client=info".parse()?)
        };

        Ok(filter)
    }

    fn init_json_logging(&self, env_filter: EnvFilter) -> Result<()> {
        let fmt_layer = fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(true)
            .with_file(self.include_location)
            .with_line_number(self.include_location)
            .with_thread_ids(self.include_thread_ids)
            .with_thread_names(true)
            .with_target(true)
            .with_span_events(if self.log_spans {
                FmtSpan::ENTER | FmtSpan::EXIT
            } else {
                FmtSpan::NONE
            });

        let fmt_layer = if self.include_timestamps {
            fmt_layer.boxed()
        } else {
            fmt_layer.without_time().boxed()
        };

        Registry::default()
            .with(env_filter)
            .with(fmt_layer)
            .try_init()?;

        Ok(())
    }

    fn init_pretty_logging(&self, env_filter: EnvFilter) -> Result<()> {
        let fmt_layer = fmt::layer()
            .pretty()
            .with_file(self.include_location)
            .with_line_number(self.include_location)
            .with_thread_ids(self.include_thread_ids)
            .with_thread_names(true)
            .with_target(false)
            .with_span_events(if self.log_spans {
                FmtSpan::ENTER | FmtSpan::EXIT
            } else {
                FmtSpan::NONE
            });

        let fmt_layer = if self.include_timestamps {
            fmt_layer.boxed()
        } else {
            fmt_layer.without_time().boxed()
        };

        Registry::default()
            .with(env_filter)
            .with(fmt_layer)
            .try_init()?;

        Ok(())
    }
}

/// Initialize default logging for the application
pub fn init_default_logging() -> Result<()> {
    let config = LoggingConfig::from_env();
    config.init_tracing()
}

/// Initialize production logging with JSON format
pub fn init_production_logging() -> Result<()> {
    let mut config = LoggingConfig::from_env();
    config.json_format = true;
    config.include_location = false; // Reduce log size in production
    config.log_spans = false;
    config.init_tracing()
}

/// Initialize development logging with pretty format
pub fn init_development_logging() -> Result<()> {
    let mut config = LoggingConfig::from_env();
    config.json_format = false;
    config.include_location = true;
    config.log_spans = true;
    config.level = Level::DEBUG;
    config.init_tracing()
}

/// Structured logging macros for common operations
#[macro_export]
macro_rules! log_operation_start {
    ($operation:expr, $($field:ident = $value:expr),*) => {
        tracing::info!(
            operation = $operation,
            status = "started",
            $($field = $value,)*
            "Operation started"
        );
    };
}

#[macro_export]
macro_rules! log_operation_success {
    ($operation:expr, $duration:expr, $($field:ident = $value:expr),*) => {
        tracing::info!(
            operation = $operation,
            status = "success",
            duration_ms = $duration.as_millis(),
            $($field = $value,)*
            "Operation completed successfully"
        );
    };
}

#[macro_export]
macro_rules! log_operation_error {
    ($operation:expr, $error:expr, $($field:ident = $value:expr),*) => {
        tracing::error!(
            operation = $operation,
            status = "error",
            error = %$error,
            $($field = $value,)*
            "Operation failed"
        );
    };
}

#[macro_export]
macro_rules! log_performance_metric {
    ($metric:expr, $value:expr, $unit:expr, $($field:ident = $tag_value:expr),*) => {
        tracing::info!(
            metric = $metric,
            value = $value,
            unit = $unit,
            $($field = $tag_value,)*
            "Performance metric"
        );
    };
}
