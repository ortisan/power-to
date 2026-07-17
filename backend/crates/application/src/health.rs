use async_trait::async_trait;
use thiserror::Error;

/// Stable application-level error for a failed readiness dependency.
///
/// Infrastructure details remain in adapter logs and are not exposed through
/// the public health response.
#[derive(Clone, Copy, Debug, Error)]
#[error("a required dependency is unavailable")]
pub struct ReadinessError;

/// Port used by an inbound adapter to determine whether it can serve traffic.
#[async_trait]
pub trait ReadinessProbe: Send + Sync {
    /// Checks all dependencies represented by this probe.
    async fn check(&self) -> Result<(), ReadinessError>;
}
