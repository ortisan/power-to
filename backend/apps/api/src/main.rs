use std::{env, net::SocketAddr, num::NonZeroU32, sync::Arc, time::Duration};

use anyhow::{Context as _, Result, bail};
use powerto_adapters::{
    account_directory::PostgresAccountDirectory,
    database::{PostgresReadiness, create_pool},
    issue_store::PostgresIssueStore,
    observability::{TelemetryConfig, init_telemetry},
    oidc::{OidcActorAuthenticator, OidcConfig},
};
use powerto_api::{ActorAuthentication, ApiState, router};
use powerto_application::issues::IssueService;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    let _telemetry_guard = init_telemetry(&TelemetryConfig {
        service_name: "powerto-api".to_owned(),
        service_version: env!("CARGO_PKG_VERSION").to_owned(),
        environment: config.environment.clone(),
        otlp_endpoint: config.otlp_endpoint.clone(),
    })
    .context("telemetry initialization failed")?;

    let pool = create_pool(
        &config.database_url,
        config.database_pool_size,
        config.database_timeout,
    )
    .await
    .context("PostgreSQL connection pool initialization failed")?;
    let actor_authentication = match config.authentication {
        AuthenticationConfig::Oidc {
            issuer,
            audience,
            clock_skew,
            http_timeout,
            jwks_refresh_interval,
            allow_insecure_loopback_http,
        } => ActorAuthentication::Oidc(Arc::new(
            OidcActorAuthenticator::discover(
                OidcConfig {
                    issuer,
                    audience,
                    clock_skew,
                    http_timeout,
                    jwks_refresh_interval,
                    allow_insecure_loopback_http,
                },
                Arc::new(PostgresAccountDirectory::new(pool.clone())),
            )
            .await
            .context("OIDC authentication initialization failed")?,
        )),
        AuthenticationConfig::InsecureLoopbackHeader => ActorAuthentication::InsecureLoopbackHeader,
        AuthenticationConfig::Disabled => ActorAuthentication::Disabled,
    };
    let state = ApiState::new(
        Arc::new(PostgresReadiness::new(
            pool.clone(),
            config.database_timeout,
        )),
        IssueService::new(
            Arc::new(PostgresIssueStore::new(pool)),
            config.privacy_notice_version.clone(),
        ),
        actor_authentication,
    );
    let listener = tokio::net::TcpListener::bind(config.http_address)
        .await
        .context("HTTP listener bind failed")?;
    let local_address = listener
        .local_addr()
        .context("HTTP listener address could not be read")?;

    tracing::info!(address = %local_address, "PowerTo API is listening");
    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("HTTP server failed")?;
    tracing::info!("PowerTo API stopped");

    Ok(())
}

struct Config {
    authentication: AuthenticationConfig,
    database_url: String,
    database_pool_size: NonZeroU32,
    database_timeout: Duration,
    environment: String,
    http_address: SocketAddr,
    otlp_endpoint: Option<String>,
    privacy_notice_version: String,
}

enum AuthenticationConfig {
    Disabled,
    InsecureLoopbackHeader,
    Oidc {
        issuer: String,
        audience: String,
        clock_skew: Duration,
        http_timeout: Duration,
        jwks_refresh_interval: Duration,
        allow_insecure_loopback_http: bool,
    },
}

impl Config {
    fn from_env() -> Result<Self> {
        let database_url = env::var("POWERTO_DATABASE_URL")
            .context("POWERTO_DATABASE_URL must be set; the value is never logged")?;
        let pool_size = env::var("POWERTO_DATABASE_POOL_SIZE")
            .unwrap_or_else(|_| "10".to_owned())
            .parse::<NonZeroU32>()
            .context("POWERTO_DATABASE_POOL_SIZE must be a positive integer")?;
        let http_address = env::var("POWERTO_HTTP_ADDRESS")
            .unwrap_or_else(|_| "127.0.0.1:8080".to_owned())
            .parse::<SocketAddr>()
            .context("POWERTO_HTTP_ADDRESS must be a socket address")?;
        let database_timeout_ms = env::var("POWERTO_DATABASE_TIMEOUT_MS")
            .unwrap_or_else(|_| "500".to_owned())
            .parse::<u64>()
            .context("POWERTO_DATABASE_TIMEOUT_MS must be a non-negative integer")?;

        let environment = env::var("POWERTO_ENVIRONMENT").unwrap_or_else(|_| "local".to_owned());
        let privacy_notice_version = match env::var("POWERTO_PRIVACY_NOTICE_VERSION") {
            Ok(value) if !value.trim().is_empty() => value,
            Ok(_) => bail!("POWERTO_PRIVACY_NOTICE_VERSION must not be blank"),
            Err(_) if environment == "local" => "privacy-v1".to_owned(),
            Err(_) => bail!("POWERTO_PRIVACY_NOTICE_VERSION must be set outside local"),
        };
        let insecure_local_actor = env::var("POWERTO_ALLOW_INSECURE_LOCAL_ACTOR_HEADER")
            .unwrap_or_else(|_| "false".to_owned())
            .parse::<bool>()
            .context("POWERTO_ALLOW_INSECURE_LOCAL_ACTOR_HEADER must be true or false")?;
        if insecure_local_actor && environment != "local" {
            bail!("the insecure local actor header is allowed only in the local environment");
        }
        if insecure_local_actor && !http_address.ip().is_loopback() {
            bail!("the insecure local actor header requires a loopback HTTP address");
        }
        let oidc_issuer = env::var("POWERTO_OIDC_ISSUER").ok();
        let oidc_audience = env::var("POWERTO_OIDC_AUDIENCE").ok();
        let authentication = match (oidc_issuer, oidc_audience) {
            (Some(issuer), Some(audience)) => {
                if issuer.trim().is_empty() || audience.trim().is_empty() {
                    bail!("POWERTO_OIDC_ISSUER and POWERTO_OIDC_AUDIENCE must not be blank");
                }
                if insecure_local_actor {
                    bail!("OIDC and the insecure local actor header cannot be enabled together");
                }
                AuthenticationConfig::Oidc {
                    issuer,
                    audience,
                    clock_skew: duration_from_env("POWERTO_OIDC_CLOCK_SKEW_SECONDS", 30, 0, 300)?,
                    http_timeout: duration_from_env("POWERTO_OIDC_HTTP_TIMEOUT_SECONDS", 3, 1, 30)?,
                    jwks_refresh_interval: duration_from_env(
                        "POWERTO_OIDC_JWKS_REFRESH_SECONDS",
                        300,
                        30,
                        86_400,
                    )?,
                    allow_insecure_loopback_http: environment == "local",
                }
            }
            (Some(_), None) | (None, Some(_)) => {
                bail!("POWERTO_OIDC_ISSUER and POWERTO_OIDC_AUDIENCE must be set together")
            }
            (None, None) if insecure_local_actor => AuthenticationConfig::InsecureLoopbackHeader,
            (None, None) if environment == "local" => AuthenticationConfig::Disabled,
            (None, None) => {
                bail!("POWERTO_OIDC_ISSUER and POWERTO_OIDC_AUDIENCE are required outside local")
            }
        };

        Ok(Self {
            authentication,
            database_url,
            database_pool_size: pool_size,
            database_timeout: Duration::from_millis(database_timeout_ms),
            environment,
            http_address,
            otlp_endpoint: env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok(),
            privacy_notice_version,
        })
    }
}

fn duration_from_env(
    name: &str,
    default_seconds: u64,
    minimum_seconds: u64,
    maximum_seconds: u64,
) -> Result<Duration> {
    let seconds = env::var(name)
        .unwrap_or_else(|_| default_seconds.to_string())
        .parse::<u64>()
        .with_context(|| format!("{name} must be a non-negative integer"))?;
    if !(minimum_seconds..=maximum_seconds).contains(&seconds) {
        bail!("{name} must be between {minimum_seconds} and {maximum_seconds}");
    }
    Ok(Duration::from_secs(seconds))
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(error = %error, "failed to install the shutdown signal handler");
    }
}
