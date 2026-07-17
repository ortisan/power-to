use std::sync::Arc;

use async_trait::async_trait;
use powerto_domain::{
    AccountId, GeoPoint, GeometrySource, Issue, IssueReference, IssueSubmission,
    IssueSubmissionInput, IssueValidationError,
};
use sha2::{Digest as _, Sha256};
use thiserror::Error;
use uuid::Uuid;

const SUBMISSION_FINGERPRINT_VERSION: i16 = 1;

/// High-entropy client key used to make mobile retries safe.
///
/// Adapters persist only its digest and must never log or echo the raw value.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct IdempotencyKey(Uuid);

impl IdempotencyKey {
    /// Wraps a UUID parsed by an inbound adapter.
    #[must_use]
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the UUID to an outer adapter for one-way hashing.
    #[must_use]
    pub const fn into_uuid(self) -> Uuid {
        self.0
    }
}

/// Primitive input for submitting a confirmed civic problem.
///
/// Account identity, server identifiers, status, timestamps, and policy state
/// are deliberately absent.
pub struct SubmitIssueCommand {
    pub idempotency_key: IdempotencyKey,
    pub title: String,
    pub category_id: String,
    pub summary: String,
    pub problem_statement: String,
    pub affected_community: String,
    pub desired_outcome: String,
    pub longitude: f64,
    pub latitude: f64,
    pub geometry_source: GeometrySource,
    pub location_confirmed: bool,
    pub location_label: Option<String>,
    pub public_attribution: bool,
    pub privacy_notice_version: String,
    pub privacy_notice_accepted: bool,
}

/// Versioned digest of the normalized citizen command used for idempotency.
///
/// It deliberately excludes identifiers, timestamps, and submission policy
/// assigned by the server. This lets an exact retry find its historical result
/// before the current deploy applies rules that may have changed meanwhile.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct SubmissionFingerprint {
    version: i16,
    digest: [u8; 32],
}

impl SubmissionFingerprint {
    /// Computes the stable v1 fingerprint without retaining citizen content.
    #[must_use]
    pub fn from_command(command: &SubmitIssueCommand) -> Self {
        let mut digest = Sha256::new();
        digest.update(b"powerto-submit-issue-fingerprint-v1\0");
        fingerprint_text(&mut digest, &command.title);
        fingerprint_text(&mut digest, &command.category_id);
        fingerprint_text(&mut digest, &command.summary);
        fingerprint_text(&mut digest, &command.problem_statement);
        fingerprint_text(&mut digest, &command.affected_community);
        fingerprint_text(&mut digest, &command.desired_outcome);
        fingerprint_coordinate(&mut digest, command.longitude);
        fingerprint_coordinate(&mut digest, command.latitude);
        fingerprint_text(&mut digest, command.geometry_source.as_str());
        digest.update([u8::from(command.location_confirmed)]);
        fingerprint_optional_text(&mut digest, command.location_label.as_deref());
        digest.update([u8::from(command.public_attribution)]);
        fingerprint_text(&mut digest, &command.privacy_notice_version);
        digest.update([u8::from(command.privacy_notice_accepted)]);

        Self {
            version: SUBMISSION_FINGERPRINT_VERSION,
            digest: digest.finalize().into(),
        }
    }

    /// Identifies the canonicalization algorithm used for the digest.
    #[must_use]
    pub const fn version(self) -> i16 {
        self.version
    }

    /// Exposes only the one-way digest to persistence adapters.
    #[must_use]
    pub const fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
}

/// Whether the response represents a new transaction or an exact replay.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmissionDisposition {
    Created,
    Replayed,
}

/// Successful issue submission result.
pub struct SubmitIssueResult {
    pub issue: Issue,
    pub disposition: SubmissionDisposition,
}

/// Result of checking an idempotency key before evaluating current policy.
pub enum ReplayLookup {
    Missing,
    Replayed(Box<Issue>),
    Conflict,
}

/// Atomic persistence outcome. A replay can win a concurrent insertion race.
pub enum PersistIssueOutcome {
    Created(Box<Issue>),
    Replayed(Box<Issue>),
    Conflict,
}

/// Stable failures exposed by the persistence port.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum IssueStoreError {
    #[error("issue storage is unavailable")]
    Unavailable,
    #[error("stored issue data violates application invariants")]
    InvalidStoredData,
}

/// Purpose-specific transaction boundary for issue intake and owner reads.
#[async_trait]
pub trait IssueStore: Send + Sync {
    /// Looks up an earlier command before current policy evaluation.
    async fn find_replay(
        &self,
        account_id: AccountId,
        key: IdempotencyKey,
        fingerprint: SubmissionFingerprint,
    ) -> Result<ReplayLookup, IssueStoreError>;

    /// Atomically reserves idempotency and writes issue, private context,
    /// audit event, and outbox message.
    async fn persist_idempotently(
        &self,
        account_id: AccountId,
        key: IdempotencyKey,
        fingerprint: SubmissionFingerprint,
        submission: &IssueSubmission,
    ) -> Result<PersistIssueOutcome, IssueStoreError>;

    /// Loads only a row owned by the authenticated account. Implementations
    /// must filter owner and opaque reference in the same query.
    async fn find_owned(
        &self,
        account_id: AccountId,
        reference: IssueReference,
    ) -> Result<Option<Issue>, IssueStoreError>;
}

/// Issue-intake use cases with no dependency on HTTP or Diesel.
#[derive(Clone)]
pub struct IssueService {
    store: Arc<dyn IssueStore>,
    current_privacy_notice_version: Arc<str>,
}

impl IssueService {
    /// Creates the service using the privacy notice accepted for new commands.
    #[must_use]
    pub fn new(
        store: Arc<dyn IssueStore>,
        current_privacy_notice_version: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            store,
            current_privacy_notice_version: current_privacy_notice_version.into(),
        }
    }

    /// Validates and submits an issue with replay-safe policy ordering.
    pub async fn submit(
        &self,
        account_id: AccountId,
        command: SubmitIssueCommand,
    ) -> Result<SubmitIssueResult, SubmitIssueError> {
        let idempotency_key = command.idempotency_key;
        let fingerprint = SubmissionFingerprint::from_command(&command);

        match self
            .store
            .find_replay(account_id, idempotency_key, fingerprint)
            .await?
        {
            ReplayLookup::Replayed(issue) => {
                return Ok(SubmitIssueResult {
                    issue: *issue,
                    disposition: SubmissionDisposition::Replayed,
                });
            }
            ReplayLookup::Conflict => return Err(SubmitIssueError::IdempotencyConflict),
            ReplayLookup::Missing => {}
        }

        let point = GeoPoint::new(command.longitude, command.latitude)?;
        let submission = IssueSubmission::new(IssueSubmissionInput {
            title: command.title,
            category_id: command.category_id,
            summary: command.summary,
            problem_statement: command.problem_statement,
            affected_community: command.affected_community,
            desired_outcome: command.desired_outcome,
            point,
            geometry_source: command.geometry_source,
            location_confirmed: command.location_confirmed,
            location_label: command.location_label,
            public_attribution: command.public_attribution,
            privacy_notice_version: command.privacy_notice_version,
            privacy_notice_accepted: command.privacy_notice_accepted,
        })?;

        if submission.privacy_notice_version() != &*self.current_privacy_notice_version {
            return Err(SubmitIssueError::PrivacyNoticeOutdated);
        }

        match self
            .store
            .persist_idempotently(account_id, idempotency_key, fingerprint, &submission)
            .await?
        {
            PersistIssueOutcome::Created(issue) => Ok(SubmitIssueResult {
                issue: *issue,
                disposition: SubmissionDisposition::Created,
            }),
            PersistIssueOutcome::Replayed(issue) => Ok(SubmitIssueResult {
                issue: *issue,
                disposition: SubmissionDisposition::Replayed,
            }),
            PersistIssueOutcome::Conflict => Err(SubmitIssueError::IdempotencyConflict),
        }
    }

    /// Finds an issue only within the authenticated account's private scope.
    pub async fn get_owned(
        &self,
        account_id: AccountId,
        reference: IssueReference,
    ) -> Result<Option<Issue>, GetOwnIssueError> {
        self.store
            .find_owned(account_id, reference)
            .await
            .map_err(Into::into)
    }
}

fn fingerprint_text(digest: &mut Sha256, value: &str) {
    let value = value.trim();
    digest.update(u64::try_from(value.len()).unwrap_or(u64::MAX).to_be_bytes());
    digest.update(value.as_bytes());
}

fn fingerprint_optional_text(digest: &mut Sha256, value: Option<&str>) {
    match value {
        Some(value) => {
            digest.update([1]);
            fingerprint_text(digest, value);
        }
        None => digest.update([0]),
    }
}

fn fingerprint_coordinate(digest: &mut Sha256, value: f64) {
    let normalized = if value == 0.0 { 0.0 } else { value };
    digest.update(normalized.to_bits().to_be_bytes());
}

/// Safe application failures for issue submission.
#[derive(Debug, Error)]
pub enum SubmitIssueError {
    #[error("issue input is invalid")]
    InvalidInput(#[from] IssueValidationError),
    #[error("the privacy notice version is no longer accepted")]
    PrivacyNoticeOutdated,
    #[error("the idempotency key was already used for a different command")]
    IdempotencyConflict,
    #[error("issue storage is unavailable")]
    Unavailable,
    #[error("stored issue data is invalid")]
    Internal,
}

impl From<IssueStoreError> for SubmitIssueError {
    fn from(value: IssueStoreError) -> Self {
        match value {
            IssueStoreError::Unavailable => Self::Unavailable,
            IssueStoreError::InvalidStoredData => Self::Internal,
        }
    }
}

/// Safe application failures for the owner-scoped issue view.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum GetOwnIssueError {
    #[error("issue storage is unavailable")]
    Unavailable,
    #[error("stored issue data is invalid")]
    Internal,
}

impl From<IssueStoreError> for GetOwnIssueError {
    fn from(value: IssueStoreError) -> Self {
        match value {
            IssueStoreError::Unavailable => Self::Unavailable,
            IssueStoreError::InvalidStoredData => Self::Internal,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use chrono::Utc;
    use powerto_domain::{AccountId, Issue, IssueReference, IssueSubmission};
    use uuid::Uuid;

    use super::{
        IdempotencyKey, IssueService, IssueStore, IssueStoreError, PersistIssueOutcome,
        ReplayLookup, SubmissionDisposition, SubmissionFingerprint, SubmitIssueCommand,
        SubmitIssueError,
    };

    struct ReplayStore {
        issue: Issue,
        fingerprint: SubmissionFingerprint,
    }

    #[async_trait]
    impl IssueStore for ReplayStore {
        async fn find_replay(
            &self,
            _account_id: AccountId,
            _key: IdempotencyKey,
            fingerprint: SubmissionFingerprint,
        ) -> Result<ReplayLookup, IssueStoreError> {
            if self.fingerprint == fingerprint {
                Ok(ReplayLookup::Replayed(Box::new(self.issue.clone())))
            } else {
                Ok(ReplayLookup::Conflict)
            }
        }

        async fn persist_idempotently(
            &self,
            _account_id: AccountId,
            _key: IdempotencyKey,
            _fingerprint: SubmissionFingerprint,
            _submission: &IssueSubmission,
        ) -> Result<PersistIssueOutcome, IssueStoreError> {
            panic!("replay must not persist again")
        }

        async fn find_owned(
            &self,
            _account_id: AccountId,
            _reference: IssueReference,
        ) -> Result<Option<Issue>, IssueStoreError> {
            Ok(None)
        }
    }

    fn command(privacy_notice_version: &str) -> SubmitIssueCommand {
        SubmitIssueCommand {
            idempotency_key: IdempotencyKey::from_uuid(Uuid::new_v4()),
            title: "Deep pothole".to_owned(),
            category_id: "road-surface".to_owned(),
            summary: "Buses avoid the damaged lane.".to_owned(),
            problem_statement: "The depression remains after repeated observations.".to_owned(),
            affected_community: "Passengers, cyclists, and drivers.".to_owned(),
            desired_outcome: "Restore a level and safe road surface.".to_owned(),
            longitude: -46.633_308,
            latitude: -23.550_52,
            geometry_source: powerto_domain::GeometrySource::MapSelection,
            location_confirmed: true,
            location_label: Some("Eastbound bus lane".to_owned()),
            public_attribution: false,
            privacy_notice_version: privacy_notice_version.to_owned(),
            privacy_notice_accepted: true,
        }
    }

    fn account_id() -> AccountId {
        AccountId::from_uuid(Uuid::new_v4())
    }

    fn issue_for(command: &SubmitIssueCommand) -> Issue {
        let point = match powerto_domain::GeoPoint::new(command.longitude, command.latitude) {
            Ok(point) => point,
            Err(error) => panic!("valid point failed: {error}"),
        };
        let submission = IssueSubmission::new(powerto_domain::IssueSubmissionInput {
            title: command.title.clone(),
            category_id: command.category_id.clone(),
            summary: command.summary.clone(),
            problem_statement: command.problem_statement.clone(),
            affected_community: command.affected_community.clone(),
            desired_outcome: command.desired_outcome.clone(),
            point,
            geometry_source: command.geometry_source,
            location_confirmed: command.location_confirmed,
            location_label: command.location_label.clone(),
            public_attribution: command.public_attribution,
            privacy_notice_version: command.privacy_notice_version.clone(),
            privacy_notice_accepted: command.privacy_notice_accepted,
        });
        match submission {
            Ok(submission) => Issue::from_submission(submission, Utc::now()),
            Err(error) => panic!("valid submission failed: {error}"),
        }
    }

    #[tokio::test]
    async fn replay_succeeds_after_current_privacy_notice_changes() {
        let original = command("privacy-v1");
        let issue = issue_for(&original);
        let fingerprint = SubmissionFingerprint::from_command(&original);
        let service = IssueService::new(Arc::new(ReplayStore { issue, fingerprint }), "privacy-v2");

        let result = service.submit(account_id(), original).await;

        match result {
            Ok(result) => assert_eq!(result.disposition, SubmissionDisposition::Replayed),
            Err(error) => panic!("valid replay failed: {error}"),
        }
    }

    #[tokio::test]
    async fn changed_command_with_same_key_is_a_conflict() {
        let original = command("privacy-v1");
        let issue = issue_for(&original);
        let fingerprint = SubmissionFingerprint::from_command(&original);
        let service = IssueService::new(Arc::new(ReplayStore { issue, fingerprint }), "privacy-v1");
        let mut changed = original;
        changed.title = "Different problem".to_owned();

        let result = service.submit(account_id(), changed).await;

        assert!(matches!(result, Err(SubmitIssueError::IdempotencyConflict)));
    }

    #[tokio::test]
    async fn historical_replay_is_loaded_before_current_domain_validation() {
        let valid = command("privacy-v1");
        let issue = issue_for(&valid);
        let mut historical = command("privacy-v1");
        historical.location_confirmed = false;
        let fingerprint = SubmissionFingerprint::from_command(&historical);
        let service = IssueService::new(Arc::new(ReplayStore { issue, fingerprint }), "privacy-v2");

        let result = service.submit(account_id(), historical).await;

        match result {
            Ok(result) => assert_eq!(result.disposition, SubmissionDisposition::Replayed),
            Err(error) => panic!("historical replay was revalidated: {error}"),
        }
    }

    #[test]
    fn fingerprint_normalizes_whitespace_and_signed_zero() {
        let mut original = command("privacy-v1");
        original.longitude = 0.0;
        let mut equivalent = command(" privacy-v1 ");
        equivalent.title = "  Deep pothole  ".to_owned();
        equivalent.longitude = -0.0;

        let original_fingerprint = SubmissionFingerprint::from_command(&original);
        let equivalent_fingerprint = SubmissionFingerprint::from_command(&equivalent);

        assert!(original_fingerprint == equivalent_fingerprint);
    }
}
