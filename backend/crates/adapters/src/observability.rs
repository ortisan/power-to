use std::time::Duration;

use opentelemetry::{KeyValue, global, trace::TracerProvider as _};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{ExporterBuildError, WithExportConfig};
use opentelemetry_sdk::{
    Resource,
    logs::SdkLoggerProvider,
    metrics::{SdkMeterProvider, Temporality},
    propagation::TraceContextPropagator,
    trace::SdkTracerProvider,
};
use thiserror::Error;
use tracing_subscriber::{
    EnvFilter,
    layer::SubscriberExt as _,
    util::{SubscriberInitExt as _, TryInitError},
};

/// Runtime metadata and the optional Collector endpoint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TelemetryConfig {
    /// Stable OTLP `service.name`.
    pub service_name: String,
    /// Build/package version exposed as `service.version`.
    pub service_version: String,
    /// Deployment environment name such as `local`, `staging`, or `production`.
    pub environment: String,
    /// OTLP/gRPC Collector endpoint. `None` keeps structured stdout only.
    pub otlp_endpoint: Option<String>,
}

/// Owns SDK providers so buffered telemetry is flushed on graceful shutdown.
#[derive(Debug, Default)]
pub struct TelemetryGuard {
    tracer_provider: Option<SdkTracerProvider>,
    meter_provider: Option<SdkMeterProvider>,
    logger_provider: Option<SdkLoggerProvider>,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Some(provider) = &self.logger_provider
            && let Err(error) = provider.shutdown()
        {
            eprintln!("failed to flush OpenTelemetry logs: {error}");
        }
        if let Some(provider) = &self.meter_provider
            && let Err(error) = provider.shutdown()
        {
            eprintln!("failed to flush OpenTelemetry metrics: {error}");
        }
        if let Some(provider) = &self.tracer_provider
            && let Err(error) = provider.shutdown()
        {
            eprintln!("failed to flush OpenTelemetry traces: {error}");
        }
    }
}

/// Initializes JSON stdout logging and, when configured, OTLP traces, metrics,
/// and logs through one OpenTelemetry Collector endpoint.
pub fn init_telemetry(config: &TelemetryConfig) -> Result<TelemetryGuard, TelemetryError> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let Some(endpoint) = &config.otlp_endpoint else {
        tracing_subscriber::registry()
            .with(filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .flatten_event(true)
                    .with_current_span(true),
            )
            .try_init()?;
        return Ok(TelemetryGuard::default());
    };

    let resource = Resource::builder()
        .with_service_name(config.service_name.clone())
        .with_attributes([
            KeyValue::new("service.version", config.service_version.clone()),
            KeyValue::new("deployment.environment.name", config.environment.clone()),
        ])
        .build();
    let export_timeout = Duration::from_secs(5);

    let span_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .with_timeout(export_timeout)
        .build()
        .map_err(TelemetryError::TraceExporter)?;
    let tracer_provider = SdkTracerProvider::builder()
        .with_resource(resource.clone())
        .with_batch_exporter(span_exporter)
        .build();
    let tracer = tracer_provider.tracer(config.service_name.clone());

    let metric_exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .with_timeout(export_timeout)
        .with_temporality(Temporality::Cumulative)
        .build()
        .map_err(TelemetryError::MetricExporter)?;
    let meter_provider = SdkMeterProvider::builder()
        .with_resource(resource.clone())
        .with_periodic_exporter(metric_exporter)
        .build();

    let log_exporter = opentelemetry_otlp::LogExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .with_timeout(export_timeout)
        .build()
        .map_err(TelemetryError::LogExporter)?;
    let logger_provider = SdkLoggerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(log_exporter)
        .build();

    global::set_text_map_propagator(TraceContextPropagator::new());
    global::set_tracer_provider(tracer_provider.clone());
    global::set_meter_provider(meter_provider.clone());

    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .flatten_event(true)
                .with_current_span(true),
        )
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .with(OpenTelemetryTracingBridge::new(&logger_provider))
        .try_init()?;

    Ok(TelemetryGuard {
        tracer_provider: Some(tracer_provider),
        meter_provider: Some(meter_provider),
        logger_provider: Some(logger_provider),
    })
}

/// Failures that prevent telemetry initialization.
#[derive(Debug, Error)]
pub enum TelemetryError {
    /// OTLP trace exporter configuration failed.
    #[error("failed to configure the OTLP trace exporter")]
    TraceExporter(#[source] ExporterBuildError),
    /// OTLP metric exporter configuration failed.
    #[error("failed to configure the OTLP metric exporter")]
    MetricExporter(#[source] ExporterBuildError),
    /// OTLP log exporter configuration failed.
    #[error("failed to configure the OTLP log exporter")]
    LogExporter(#[source] ExporterBuildError),
    /// A process-global tracing subscriber was already installed.
    #[error("failed to install the global tracing subscriber")]
    Subscriber(#[from] TryInitError),
}
