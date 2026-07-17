use std::{env, num::NonZeroU32, time::Duration};

use anyhow::{Context as _, Result};
use powerto_adapters::{
    database::{PostgresReadiness, create_pool},
    observability::{TelemetryConfig, init_telemetry},
};
use powerto_application::health::ReadinessProbe as _;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    let _telemetry_guard = init_telemetry(&TelemetryConfig {
        service_name: "powerto-worker".to_owned(),
        service_version: env!("CARGO_PKG_VERSION").to_owned(),
        environment: config.environment,
        otlp_endpoint: config.otlp_endpoint,
    })
    .context("telemetry initialization failed")?;

    let pool = create_pool(
        &config.database_url,
        config.database_pool_size,
        config.database_timeout,
    )
    .await
    .context("PostgreSQL connection pool initialization failed")?;
    PostgresReadiness::new(pool, config.database_timeout)
        .check()
        .await
        .context("PostgreSQL readiness check failed")?;

    tracing::info!("PowerTo worker is ready; job handlers will arrive with vertical slices");
    tokio::signal::ctrl_c()
        .await
        .context("worker shutdown signal handler failed")?;
    tracing::info!("PowerTo worker stopped");

    Ok(())
}

#[derive(Debug)]
struct Config {
    database_url: String,
    database_pool_size: NonZeroU32,
    database_timeout: Duration,
    environment: String,
    otlp_endpoint: Option<String>,
}

impl Config {
    fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: env::var("POWERTO_DATABASE_URL")
                .context("POWERTO_DATABASE_URL must be set; the value is never logged")?,
            database_pool_size: env::var("POWERTO_DATABASE_POOL_SIZE")
                .unwrap_or_else(|_| "10".to_owned())
                .parse::<NonZeroU32>()
                .context("POWERTO_DATABASE_POOL_SIZE must be a positive integer")?,
            database_timeout: Duration::from_millis(
                env::var("POWERTO_DATABASE_TIMEOUT_MS")
                    .unwrap_or_else(|_| "500".to_owned())
                    .parse::<u64>()
                    .context("POWERTO_DATABASE_TIMEOUT_MS must be a non-negative integer")?,
            ),
            environment: env::var("POWERTO_ENVIRONMENT").unwrap_or_else(|_| "local".to_owned()),
            otlp_endpoint: env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok(),
        })
    }
}
