//! Authentication and local-account resolution ports.
//!
//! Tokens and provider-specific claims remain opaque at this boundary. Outer
//! adapters authenticate a credential and return only the internal account
//! identifier required by application use cases.

use async_trait::async_trait;
use powerto_domain::AccountId;
use thiserror::Error;

const MAX_ISSUER_LENGTH: usize = 2_048;
const MAX_SUBJECT_LENGTH: usize = 255;

/// Opaque credential presented to an authentication adapter.
///
/// Deliberately does not implement `Debug`, `Display`, `Clone`, or serialization
/// so bearer tokens cannot be copied into diagnostics accidentally.
pub struct PresentedCredential(String);

impl PresentedCredential {
    /// Wraps a credential extracted at the HTTP boundary.
    #[must_use]
    pub fn new(value: String) -> Self {
        Self(value)
    }

    /// Exposes the credential only to the selected authentication adapter.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

/// Stable external identity after cryptographic authentication.
///
/// This type deliberately does not implement `Debug` or `Display`; issuer and
/// subject are correlation identifiers and must not enter telemetry.
pub struct ExternalIdentity {
    issuer: String,
    subject: String,
}

impl ExternalIdentity {
    /// Validates identity data before it reaches persistence.
    pub fn new(issuer: String, subject: String) -> Result<Self, InvalidExternalIdentity> {
        if issuer.trim().is_empty()
            || issuer.trim() != issuer
            || issuer.len() > MAX_ISSUER_LENGTH
            || subject.trim().is_empty()
            || subject.trim() != subject
            || subject.len() > MAX_SUBJECT_LENGTH
        {
            return Err(InvalidExternalIdentity);
        }
        Ok(Self { issuer, subject })
    }

    /// Returns the exact OIDC issuer value.
    #[must_use]
    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    /// Returns the issuer-local OIDC subject.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }
}

/// Invalid or unsafe external identity data.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
#[error("external identity is invalid")]
pub struct InvalidExternalIdentity;

/// Actor resolved from an authenticated external identity.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct AuthenticatedActor {
    account_id: AccountId,
}

impl AuthenticatedActor {
    /// Creates an authenticated actor from a trusted local mapping.
    #[must_use]
    pub const fn new(account_id: AccountId) -> Self {
        Self { account_id }
    }

    /// Returns the internal account identifier used by application policies.
    #[must_use]
    pub const fn account_id(self) -> AccountId {
        self.account_id
    }
}

/// Safe authentication failures exposed to inbound adapters.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum AuthenticationError {
    /// The credential is missing, malformed, expired, or not trusted.
    #[error("credential is invalid")]
    InvalidCredential,
    /// The identity is valid but its local account cannot act.
    #[error("account is not allowed to act")]
    Forbidden,
    /// Identity verification or account persistence is temporarily unavailable.
    #[error("authentication service is unavailable")]
    Unavailable,
}

/// Authentication port consumed by HTTP and other inbound adapters.
#[async_trait]
pub trait ActorAuthenticator: Send + Sync {
    /// Authenticates a credential and resolves its local actor.
    async fn authenticate(
        &self,
        credential: &PresentedCredential,
    ) -> Result<AuthenticatedActor, AuthenticationError>;
}

/// Safe local-account directory failures.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum AccountDirectoryError {
    /// The mapped local account is suspended or closed.
    #[error("account is not allowed to act")]
    Forbidden,
    /// The account directory is temporarily unavailable.
    #[error("account directory is unavailable")]
    Unavailable,
    /// Stored identity data violates an invariant.
    #[error("stored identity data is invalid")]
    InvalidStoredData,
}

/// Resolves a verified provider identity to a stable local account.
#[async_trait]
pub trait AccountDirectory: Send + Sync {
    /// Resolves an existing mapping or provisions it atomically on first use.
    async fn resolve_or_provision(
        &self,
        identity: &ExternalIdentity,
    ) -> Result<AccountId, AccountDirectoryError>;
}

#[cfg(test)]
mod tests {
    use super::{ExternalIdentity, MAX_ISSUER_LENGTH, MAX_SUBJECT_LENGTH};

    #[test]
    fn external_identity_rejects_blank_or_oversized_values() {
        assert!(ExternalIdentity::new(String::new(), "subject".to_owned()).is_err());
        assert!(ExternalIdentity::new("issuer".to_owned(), " ".to_owned()).is_err());
        assert!(ExternalIdentity::new("issuer".to_owned(), " subject ".to_owned()).is_err());
        assert!(
            ExternalIdentity::new("i".repeat(MAX_ISSUER_LENGTH + 1), "subject".to_owned()).is_err()
        );
        assert!(
            ExternalIdentity::new("issuer".to_owned(), "s".repeat(MAX_SUBJECT_LENGTH + 1)).is_err()
        );
    }
}
