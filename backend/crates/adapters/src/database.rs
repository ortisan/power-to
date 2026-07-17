use std::{num::NonZeroU32, time::Duration};

use async_trait::async_trait;
use diesel::{dsl::sql, sql_types::Integer};
use diesel_async::{
    AsyncPgConnection, RunQueryDsl,
    pooled_connection::{AsyncDieselConnectionManager, bb8},
};
use powerto_application::health::{ReadinessError, ReadinessProbe};
use thiserror::Error;

/// Asynchronous PostgreSQL pool backed by `diesel-async` and bb8.
pub type PgPool = bb8::Pool<AsyncPgConnection>;

/// Opaque pool initialization failure that cannot expose a database URL.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
#[error("PostgreSQL connection pool could not be initialized")]
pub struct DatabasePoolError;

/// Creates and verifies a PostgreSQL connection pool.
///
/// Schema migration is intentionally absent: Atlas is the only migration
/// engine and runs as a serialized release step.
pub async fn create_pool(
    database_url: &str,
    max_size: NonZeroU32,
    connection_timeout: Duration,
) -> Result<PgPool, DatabasePoolError> {
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    bb8::Pool::builder()
        .max_size(max_size.get())
        .connection_timeout(connection_timeout)
        .build(manager)
        .await
        .map_err(|_| DatabasePoolError)
}

/// PostgreSQL implementation of the application's readiness port.
#[derive(Clone, Debug)]
pub struct PostgresReadiness {
    pool: PgPool,
    timeout: Duration,
}

impl PostgresReadiness {
    /// Creates a readiness probe over an existing pool.
    #[must_use]
    pub const fn new(pool: PgPool, timeout: Duration) -> Self {
        Self { pool, timeout }
    }

    async fn check_database(&self) -> Result<(), ReadinessError> {
        let mut connection = self.pool.get().await.map_err(|_| {
            tracing::warn!("PostgreSQL readiness connection failed");
            ReadinessError
        })?;

        diesel::select(sql::<Integer>("1"))
            .get_result::<i32>(&mut connection)
            .await
            .map(|_| ())
            .map_err(|_| {
                tracing::warn!("PostgreSQL readiness query failed");
                ReadinessError
            })
    }
}

#[async_trait]
impl ReadinessProbe for PostgresReadiness {
    async fn check(&self) -> Result<(), ReadinessError> {
        tokio::time::timeout(self.timeout, self.check_database())
            .await
            .map_err(|_| {
                tracing::warn!("PostgreSQL readiness check timed out");
                ReadinessError
            })?
    }
}
