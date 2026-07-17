//! Test doubles shared across workspace crates.

use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard},
};

use async_trait::async_trait;
use chrono::Utc;
use powerto_application::health::{ReadinessError, ReadinessProbe};
use powerto_application::identity::{
    ActorAuthenticator, AuthenticatedActor, AuthenticationError, PresentedCredential,
};
use powerto_application::issues::{
    IdempotencyKey, IssueStore, IssueStoreError, PersistIssueOutcome, ReplayLookup,
    SubmissionFingerprint,
};
use powerto_domain::{AccountId, Issue, IssueReference, IssueSubmission};

/// Authentication double with a deterministic safe result.
pub struct FixedAuthenticator {
    result: Result<AuthenticatedActor, AuthenticationError>,
}

impl FixedAuthenticator {
    /// Always authenticates as the supplied local account.
    #[must_use]
    pub const fn authenticated(account_id: AccountId) -> Self {
        Self {
            result: Ok(AuthenticatedActor::new(account_id)),
        }
    }

    /// Always returns the supplied authentication failure.
    #[must_use]
    pub const fn failing(error: AuthenticationError) -> Self {
        Self { result: Err(error) }
    }
}

#[async_trait]
impl ActorAuthenticator for FixedAuthenticator {
    async fn authenticate(
        &self,
        _credential: &PresentedCredential,
    ) -> Result<AuthenticatedActor, AuthenticationError> {
        self.result
    }
}

/// Readiness double with a deterministic result.
#[derive(Clone, Copy, Debug)]
pub struct FixedReadiness {
    ready: bool,
}

impl FixedReadiness {
    /// Creates a ready dependency double.
    #[must_use]
    pub const fn ready() -> Self {
        Self { ready: true }
    }

    /// Creates an unavailable dependency double.
    #[must_use]
    pub const fn unavailable() -> Self {
        Self { ready: false }
    }
}

#[async_trait]
impl ReadinessProbe for FixedReadiness {
    async fn check(&self) -> Result<(), ReadinessError> {
        if self.ready {
            Ok(())
        } else {
            Err(ReadinessError)
        }
    }
}

/// Deterministic in-process issue store for HTTP and application boundary
/// tests. It intentionally models idempotency and owner-scoped reads.
#[derive(Default)]
pub struct InMemoryIssueStore {
    state: Mutex<InMemoryIssueState>,
}

#[derive(Default)]
struct InMemoryIssueState {
    by_command: HashMap<(AccountId, IdempotencyKey), (SubmissionFingerprint, Issue)>,
    by_reference: HashMap<IssueReference, (AccountId, Issue)>,
}

impl InMemoryIssueStore {
    fn lock(&self) -> Result<MutexGuard<'_, InMemoryIssueState>, IssueStoreError> {
        self.state.lock().map_err(|_| IssueStoreError::Unavailable)
    }
}

#[async_trait]
impl IssueStore for InMemoryIssueStore {
    async fn find_replay(
        &self,
        account_id: AccountId,
        key: IdempotencyKey,
        fingerprint: SubmissionFingerprint,
    ) -> Result<ReplayLookup, IssueStoreError> {
        let state = self.lock()?;
        Ok(match state.by_command.get(&(account_id, key)) {
            Some((stored_fingerprint, issue)) if *stored_fingerprint == fingerprint => {
                ReplayLookup::Replayed(Box::new(issue.clone()))
            }
            Some(_) => ReplayLookup::Conflict,
            None => ReplayLookup::Missing,
        })
    }

    async fn persist_idempotently(
        &self,
        account_id: AccountId,
        key: IdempotencyKey,
        fingerprint: SubmissionFingerprint,
        submission: &IssueSubmission,
    ) -> Result<PersistIssueOutcome, IssueStoreError> {
        let mut state = self.lock()?;
        if let Some((stored_fingerprint, issue)) = state.by_command.get(&(account_id, key)) {
            return Ok(if *stored_fingerprint == fingerprint {
                PersistIssueOutcome::Replayed(Box::new(issue.clone()))
            } else {
                PersistIssueOutcome::Conflict
            });
        }

        let issue = Issue::from_submission(submission.clone(), Utc::now());
        state
            .by_command
            .insert((account_id, key), (fingerprint, issue.clone()));
        state
            .by_reference
            .insert(issue.reference(), (account_id, issue.clone()));
        Ok(PersistIssueOutcome::Created(Box::new(issue)))
    }

    async fn find_owned(
        &self,
        account_id: AccountId,
        reference: IssueReference,
    ) -> Result<Option<Issue>, IssueStoreError> {
        let state = self.lock()?;
        Ok(state
            .by_reference
            .get(&reference)
            .filter(|(owner, _)| *owner == account_id)
            .map(|(_, issue)| issue.clone()))
    }
}
