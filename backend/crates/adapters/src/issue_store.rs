use async_trait::async_trait;
use chrono::{DateTime, Utc};
use diesel::{
    OptionalExtension as _, QueryableByName,
    result::Error as DieselError,
    sql_query,
    sql_types::{BigInt, Bool, Bytea, Double, Jsonb, Nullable, Text, Timestamptz, Uuid as SqlUuid},
};
use diesel_async::{AsyncConnection as _, AsyncPgConnection, RunQueryDsl as _};
use powerto_application::issues::{
    IdempotencyKey, IssueStore, IssueStoreError, PersistIssueOutcome, ReplayLookup,
    SubmissionFingerprint,
};
use powerto_domain::{
    AccountId, GeoPoint, GeometrySource, Issue, IssueId, IssueReference, IssueStatus,
    IssueSubmission,
};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

use crate::database::PgPool;

const OPERATION_VERSION: i16 = 1;
const AUDIT_CANONICAL_FORMAT: i16 = 1;
const AUDIT_STREAM_TYPE: &str = "issue";
const AUDIT_EVENT_TYPE: &str = "issue.submitted.v1";
const OUTBOX_TOPIC: &str = "civic.issue.submitted.v1";

/// Diesel/PostgreSQL implementation of the purpose-specific issue store.
#[derive(Clone, Debug)]
pub struct PostgresIssueStore {
    pool: PgPool,
}

impl PostgresIssueStore {
    /// Uses the existing application pool. Migrations remain Atlas-owned.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn connection(
        &self,
    ) -> Result<
        diesel_async::pooled_connection::bb8::PooledConnection<'_, AsyncPgConnection>,
        IssueStoreError,
    > {
        self.pool.get().await.map_err(|_| {
            tracing::warn!("issue store could not acquire a PostgreSQL connection");
            IssueStoreError::Unavailable
        })
    }
}

#[async_trait]
impl IssueStore for PostgresIssueStore {
    async fn find_replay(
        &self,
        account_id: AccountId,
        key: IdempotencyKey,
        fingerprint: SubmissionFingerprint,
    ) -> Result<ReplayLookup, IssueStoreError> {
        let mut connection = self.connection().await?;
        let key_hash = idempotency_key_hash(key);
        let existing = load_idempotency(&mut connection, account_id, &key_hash)
            .await
            .map_err(|error| database_error("load issue idempotency", error))?;

        let Some(existing) = existing else {
            return Ok(ReplayLookup::Missing);
        };
        if existing.fingerprint_version != fingerprint.version()
            || existing.request_fingerprint.as_slice() != fingerprint.digest()
        {
            return Ok(ReplayLookup::Conflict);
        }

        let row = load_owned_by_id(&mut connection, account_id, existing.issue_id)
            .await
            .map_err(|error| database_error("load replayed issue", error))?;
        row_to_issue(row).map(|issue| ReplayLookup::Replayed(Box::new(issue)))
    }

    async fn persist_idempotently(
        &self,
        account_id: AccountId,
        key: IdempotencyKey,
        fingerprint: SubmissionFingerprint,
        submission: &IssueSubmission,
    ) -> Result<PersistIssueOutcome, IssueStoreError> {
        let mut connection = self.connection().await?;
        let key_hash = idempotency_key_hash(key);
        let outcome = connection
            .transaction(async |connection| {
                persist_transaction(connection, account_id, &key_hash, fingerprint, submission)
                    .await
            })
            .await
            .map_err(|error| database_error("persist issue transaction", error))?;

        match outcome {
            TransactionOutcome::Created(row) => {
                row_to_issue(row).map(|issue| PersistIssueOutcome::Created(Box::new(issue)))
            }
            TransactionOutcome::Replayed(row) => {
                row_to_issue(row).map(|issue| PersistIssueOutcome::Replayed(Box::new(issue)))
            }
            TransactionOutcome::Conflict => Ok(PersistIssueOutcome::Conflict),
        }
    }

    async fn find_owned(
        &self,
        account_id: AccountId,
        reference: IssueReference,
    ) -> Result<Option<Issue>, IssueStoreError> {
        let mut connection = self.connection().await?;
        let row = load_owned_by_reference(&mut connection, account_id, reference)
            .await
            .map_err(|error| database_error("load owner-scoped issue", error))?;
        row.map(row_to_issue).transpose()
    }
}

enum TransactionOutcome {
    Created(IssueRow),
    Replayed(IssueRow),
    Conflict,
}

async fn persist_transaction(
    connection: &mut AsyncPgConnection,
    account_id: AccountId,
    key_hash: &[u8],
    fingerprint: SubmissionFingerprint,
    submission: &IssueSubmission,
) -> Result<TransactionOutcome, DieselError> {
    let reserved = sql_query(
        "INSERT INTO private.issue_submission_idempotency (
             submitted_by,
             operation_version,
             idempotency_key_hash,
             request_fingerprint,
             fingerprint_version,
             issue_id
         ) VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (submitted_by, operation_version, idempotency_key_hash)
         DO NOTHING
         RETURNING issue_id",
    )
    .bind::<SqlUuid, _>(account_id.into_uuid())
    .bind::<diesel::sql_types::SmallInt, _>(OPERATION_VERSION)
    .bind::<Bytea, _>(key_hash)
    .bind::<Bytea, _>(fingerprint.digest().as_slice())
    .bind::<diesel::sql_types::SmallInt, _>(fingerprint.version())
    .bind::<SqlUuid, _>(submission.id().into_uuid())
    .get_result::<ReservedIssueRow>(connection)
    .await
    .optional()?;

    if reserved.is_none() {
        let existing = load_idempotency(connection, account_id, key_hash)
            .await?
            .ok_or(DieselError::NotFound)?;
        if existing.fingerprint_version != fingerprint.version()
            || existing.request_fingerprint.as_slice() != fingerprint.digest()
        {
            return Ok(TransactionOutcome::Conflict);
        }
        let row = load_owned_by_id(connection, account_id, existing.issue_id).await?;
        return Ok(TransactionOutcome::Replayed(row));
    }

    let occurred_at = sql_query("SELECT transaction_timestamp() AS occurred_at")
        .get_result::<TransactionTimeRow>(connection)
        .await?
        .occurred_at;

    insert_issue(connection, submission, occurred_at).await?;
    insert_private_context(connection, account_id, submission, occurred_at).await?;

    let event_id = Uuid::now_v7();
    let canonical_event = canonical_issue_submitted_event(
        event_id,
        account_id,
        submission,
        fingerprint.digest(),
        occurred_at,
    );
    insert_audit_event(
        connection,
        event_id,
        account_id,
        submission.id(),
        &canonical_event,
        occurred_at,
    )
    .await?;
    insert_outbox_message(connection, event_id, submission.id(), occurred_at).await?;

    let row = load_owned_by_id(connection, account_id, submission.id().into_uuid()).await?;
    Ok(TransactionOutcome::Created(row))
}

async fn insert_issue(
    connection: &mut AsyncPgConnection,
    submission: &IssueSubmission,
    occurred_at: DateTime<Utc>,
) -> Result<(), DieselError> {
    sql_query(
        "INSERT INTO civic.issues (
             issue_id,
             public_reference,
             category_key,
             submission_policy_version,
             title,
             summary,
             problem_statement,
             affected_community,
             desired_outcome,
             public_location,
             public_location_method,
             location_label,
             submitted_at,
             updated_at
         ) VALUES (
             $1, $2, $3, $4, $5, $6, $7, $8, $9,
             public.ST_SetSRID(public.ST_MakePoint($10, $11), 4326)::public.geography,
             'exact-civic-problem-point-v1', $12, $13, $13
         )",
    )
    .bind::<SqlUuid, _>(submission.id().into_uuid())
    .bind::<SqlUuid, _>(submission.reference().into_uuid())
    .bind::<Text, _>(submission.category_id())
    .bind::<Text, _>(submission.submission_policy_version())
    .bind::<Text, _>(submission.title())
    .bind::<Text, _>(submission.summary())
    .bind::<Text, _>(submission.problem_statement())
    .bind::<Text, _>(submission.affected_community())
    .bind::<Text, _>(submission.desired_outcome())
    .bind::<Double, _>(submission.point().longitude())
    .bind::<Double, _>(submission.point().latitude())
    .bind::<Nullable<Text>, _>(submission.location_label())
    .bind::<Timestamptz, _>(occurred_at)
    .execute(connection)
    .await
    .map(|_| ())
}

async fn insert_private_context(
    connection: &mut AsyncPgConnection,
    account_id: AccountId,
    submission: &IssueSubmission,
    occurred_at: DateTime<Utc>,
) -> Result<(), DieselError> {
    sql_query(
        "INSERT INTO private.issue_submission_context (
             issue_id,
             submitted_by,
             observed_location,
             geometry_source,
             public_attribution_consent,
             privacy_notice_version,
             created_at
         ) VALUES (
             $1, $2,
             public.ST_SetSRID(public.ST_MakePoint($3, $4), 4326)::public.geography,
             $5, $6, $7, $8
         )",
    )
    .bind::<SqlUuid, _>(submission.id().into_uuid())
    .bind::<SqlUuid, _>(account_id.into_uuid())
    .bind::<Double, _>(submission.point().longitude())
    .bind::<Double, _>(submission.point().latitude())
    .bind::<Text, _>(submission.geometry_source().as_str())
    .bind::<Bool, _>(submission.public_attribution())
    .bind::<Text, _>(submission.privacy_notice_version())
    .bind::<Timestamptz, _>(occurred_at)
    .execute(connection)
    .await
    .map(|_| ())
}

async fn insert_audit_event(
    connection: &mut AsyncPgConnection,
    event_id: Uuid,
    account_id: AccountId,
    issue_id: IssueId,
    canonical_event: &[u8],
    occurred_at: DateTime<Utc>,
) -> Result<(), DieselError> {
    sql_query(
        "INSERT INTO audit.events (
             event_id,
             stream_type,
             stream_id,
             stream_version,
             event_type,
             actor_id,
             canonical_format,
             canonical_event,
             occurred_at
         ) VALUES ($1, $2, $3, 1, $4, $5, $6, $7, $8)",
    )
    .bind::<SqlUuid, _>(event_id)
    .bind::<Text, _>(AUDIT_STREAM_TYPE)
    .bind::<SqlUuid, _>(issue_id.into_uuid())
    .bind::<Text, _>(AUDIT_EVENT_TYPE)
    .bind::<SqlUuid, _>(account_id.into_uuid())
    .bind::<diesel::sql_types::SmallInt, _>(AUDIT_CANONICAL_FORMAT)
    .bind::<Bytea, _>(canonical_event)
    .bind::<Timestamptz, _>(occurred_at)
    .execute(connection)
    .await
    .map(|_| ())
}

async fn insert_outbox_message(
    connection: &mut AsyncPgConnection,
    event_id: Uuid,
    issue_id: IssueId,
    occurred_at: DateTime<Utc>,
) -> Result<(), DieselError> {
    let payload = json!({
        "event_type": AUDIT_EVENT_TYPE,
        "issue_id": issue_id.into_uuid().to_string(),
    });
    sql_query(
        "INSERT INTO jobs.outbox_messages (
             message_id,
             audit_event_id,
             topic,
             payload_version,
             payload,
             created_at,
             available_at
         ) VALUES ($1, $2, $3, 1, $4, $5, $5)",
    )
    .bind::<SqlUuid, _>(Uuid::now_v7())
    .bind::<SqlUuid, _>(event_id)
    .bind::<Text, _>(OUTBOX_TOPIC)
    .bind::<Jsonb, Value>(payload)
    .bind::<Timestamptz, _>(occurred_at)
    .execute(connection)
    .await
    .map(|_| ())
}

async fn load_idempotency(
    connection: &mut AsyncPgConnection,
    account_id: AccountId,
    key_hash: &[u8],
) -> Result<Option<IdempotencyRow>, DieselError> {
    sql_query(
        "SELECT issue_id, request_fingerprint, fingerprint_version
         FROM private.issue_submission_idempotency
         WHERE submitted_by = $1
           AND operation_version = $2
           AND idempotency_key_hash = $3",
    )
    .bind::<SqlUuid, _>(account_id.into_uuid())
    .bind::<diesel::sql_types::SmallInt, _>(OPERATION_VERSION)
    .bind::<Bytea, _>(key_hash)
    .get_result::<IdempotencyRow>(connection)
    .await
    .optional()
}

async fn load_owned_by_id(
    connection: &mut AsyncPgConnection,
    account_id: AccountId,
    issue_id: Uuid,
) -> Result<IssueRow, DieselError> {
    issue_row_query(
        "WHERE context.submitted_by = $1 AND issue.issue_id = $2",
        connection,
        account_id,
        issue_id,
    )
    .await?
    .ok_or(DieselError::NotFound)
}

async fn load_owned_by_reference(
    connection: &mut AsyncPgConnection,
    account_id: AccountId,
    reference: IssueReference,
) -> Result<Option<IssueRow>, DieselError> {
    issue_row_query(
        "WHERE context.submitted_by = $1 AND issue.public_reference = $2",
        connection,
        account_id,
        reference.into_uuid(),
    )
    .await
}

async fn issue_row_query(
    predicate: &str,
    connection: &mut AsyncPgConnection,
    account_id: AccountId,
    identifier: Uuid,
) -> Result<Option<IssueRow>, DieselError> {
    let query = format!(
        "SELECT
             issue.issue_id,
             issue.public_reference,
             issue.version,
             issue.status,
             issue.category_key,
             issue.submission_policy_version,
             issue.title,
             issue.summary,
             issue.problem_statement,
             issue.affected_community,
             issue.desired_outcome,
             issue.location_label,
             issue.submitted_at,
             public.ST_X(context.observed_location::public.geometry) AS longitude,
             public.ST_Y(context.observed_location::public.geometry) AS latitude,
             context.geometry_source,
             context.public_attribution_consent,
             context.privacy_notice_version
         FROM civic.issues AS issue
         INNER JOIN private.issue_submission_context AS context
             ON context.issue_id = issue.issue_id
         {predicate}"
    );
    sql_query(query)
        .bind::<SqlUuid, _>(account_id.into_uuid())
        .bind::<SqlUuid, _>(identifier)
        .get_result::<IssueRow>(connection)
        .await
        .optional()
}

fn row_to_issue(row: IssueRow) -> Result<Issue, IssueStoreError> {
    let version = u64::try_from(row.version).map_err(|_| invalid_stored_data())?;
    let status = row
        .status
        .parse::<IssueStatus>()
        .map_err(|_| invalid_stored_data())?;
    let geometry_source = row
        .geometry_source
        .parse::<GeometrySource>()
        .map_err(|_| invalid_stored_data())?;
    let point = GeoPoint::new(row.longitude, row.latitude).map_err(|_| invalid_stored_data())?;

    Issue::rehydrate(
        IssueId::from_uuid(row.issue_id),
        IssueReference::from_uuid(row.public_reference),
        row.title,
        row.category_key,
        row.summary,
        row.problem_statement,
        row.affected_community,
        row.desired_outcome,
        point,
        geometry_source,
        row.location_label,
        row.public_attribution_consent,
        row.privacy_notice_version,
        row.submission_policy_version,
        status,
        row.submitted_at,
        version,
    )
    .map_err(|_| invalid_stored_data())
}

fn invalid_stored_data() -> IssueStoreError {
    tracing::error!("persisted issue failed invariant validation");
    IssueStoreError::InvalidStoredData
}

fn database_error(operation: &'static str, _error: DieselError) -> IssueStoreError {
    tracing::error!(operation, "PostgreSQL issue operation failed");
    IssueStoreError::Unavailable
}

fn idempotency_key_hash(key: IdempotencyKey) -> Vec<u8> {
    Sha256::digest(key.into_uuid().as_bytes()).to_vec()
}

fn canonical_issue_submitted_event(
    event_id: Uuid,
    account_id: AccountId,
    submission: &IssueSubmission,
    request_fingerprint: &[u8],
    occurred_at: DateTime<Utc>,
) -> Vec<u8> {
    let mut canonical = Vec::with_capacity(192);
    canonical.extend_from_slice(b"powerto-audit-event-v1\0");
    canonical.extend_from_slice(event_id.as_bytes());
    canonical.extend_from_slice(submission.id().into_uuid().as_bytes());
    canonical.extend_from_slice(account_id.into_uuid().as_bytes());
    canonical.extend_from_slice(&1_u64.to_be_bytes());
    canonical.extend_from_slice(&occurred_at.timestamp_micros().to_be_bytes());
    fingerprint_text_bytes(&mut canonical, AUDIT_STREAM_TYPE);
    fingerprint_text_bytes(&mut canonical, AUDIT_EVENT_TYPE);
    canonical.extend_from_slice(request_fingerprint);
    canonical
}

fn fingerprint_text_bytes(output: &mut Vec<u8>, value: &str) {
    output.extend_from_slice(&u64::try_from(value.len()).unwrap_or(u64::MAX).to_be_bytes());
    output.extend_from_slice(value.as_bytes());
}

#[derive(QueryableByName)]
struct ReservedIssueRow {
    #[diesel(sql_type = SqlUuid)]
    #[allow(dead_code)]
    issue_id: Uuid,
}

#[derive(QueryableByName)]
struct TransactionTimeRow {
    #[diesel(sql_type = Timestamptz)]
    occurred_at: DateTime<Utc>,
}

#[derive(QueryableByName)]
struct IdempotencyRow {
    #[diesel(sql_type = SqlUuid)]
    issue_id: Uuid,
    #[diesel(sql_type = Bytea)]
    request_fingerprint: Vec<u8>,
    #[diesel(sql_type = diesel::sql_types::SmallInt)]
    fingerprint_version: i16,
}

#[derive(QueryableByName)]
struct IssueRow {
    #[diesel(sql_type = SqlUuid)]
    issue_id: Uuid,
    #[diesel(sql_type = SqlUuid)]
    public_reference: Uuid,
    #[diesel(sql_type = BigInt)]
    version: i64,
    #[diesel(sql_type = Text)]
    status: String,
    #[diesel(sql_type = Text)]
    category_key: String,
    #[diesel(sql_type = Text)]
    submission_policy_version: String,
    #[diesel(sql_type = Text)]
    title: String,
    #[diesel(sql_type = Text)]
    summary: String,
    #[diesel(sql_type = Text)]
    problem_statement: String,
    #[diesel(sql_type = Text)]
    affected_community: String,
    #[diesel(sql_type = Text)]
    desired_outcome: String,
    #[diesel(sql_type = Nullable<Text>)]
    location_label: Option<String>,
    #[diesel(sql_type = Timestamptz)]
    submitted_at: DateTime<Utc>,
    #[diesel(sql_type = Double)]
    longitude: f64,
    #[diesel(sql_type = Double)]
    latitude: f64,
    #[diesel(sql_type = Text)]
    geometry_source: String,
    #[diesel(sql_type = Bool)]
    public_attribution_consent: bool,
    #[diesel(sql_type = Text)]
    privacy_notice_version: String,
}
