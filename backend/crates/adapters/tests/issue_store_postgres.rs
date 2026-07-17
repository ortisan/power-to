use std::{env, num::NonZeroU32, sync::Arc, time::Duration};

use diesel::{QueryableByName, sql_query, sql_types::BigInt};
use diesel_async::RunQueryDsl as _;
use powerto_adapters::{database::create_pool, issue_store::PostgresIssueStore};
use powerto_application::issues::{
    IdempotencyKey, IssueService, SubmissionDisposition, SubmitIssueCommand, SubmitIssueError,
};
use powerto_domain::{AccountId, GeometrySource};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires POWERTO_TEST_DATABASE_URL pointing at a disposable migrated database"]
async fn issue_intake_is_atomic_idempotent_and_owner_scoped() {
    let database_url = match env::var("POWERTO_TEST_DATABASE_URL") {
        Ok(database_url) => database_url,
        Err(error) => panic!("POWERTO_TEST_DATABASE_URL is required: {error}"),
    };
    let pool_size = match NonZeroU32::new(4) {
        Some(pool_size) => pool_size,
        None => panic!("integration pool size must be positive"),
    };
    let pool = match create_pool(&database_url, pool_size, Duration::from_secs(30)).await {
        Ok(pool) => pool,
        Err(error) => panic!("test pool could not be created: {error}"),
    };
    let service = Arc::new(IssueService::new(
        Arc::new(PostgresIssueStore::new(pool.clone())),
        "privacy-v1",
    ));
    let account_id = AccountId::from_uuid(Uuid::new_v4());
    let other_account_id = AccountId::from_uuid(Uuid::new_v4());
    let key = IdempotencyKey::from_uuid(Uuid::new_v4());

    let first = service.submit(account_id, command(key, "Deep pothole"));
    let second = service.submit(account_id, command(key, "Deep pothole"));
    let (first, second) = tokio::join!(first, second);
    let first = successful(first);
    let second = successful(second);

    assert_ne!(first.disposition, second.disposition);
    assert!([first.disposition, second.disposition].contains(&SubmissionDisposition::Created));
    assert!([first.disposition, second.disposition].contains(&SubmissionDisposition::Replayed));
    assert_eq!(first.issue.id(), second.issue.id());
    assert!(first.issue.reference() == second.issue.reference());
    assert_eq!(first.issue.reference().into_uuid().get_version_num(), 4);
    assert_eq!(first.issue.id().into_uuid().get_version_num(), 7);

    let owner = service.get_owned(account_id, first.issue.reference()).await;
    match owner {
        Ok(Some(issue)) => assert!(issue.reference() == first.issue.reference()),
        Ok(None) => panic!("owner-scoped issue was not found"),
        Err(error) => panic!("owner-scoped read failed: {error}"),
    }
    let outsider = service
        .get_owned(other_account_id, first.issue.reference())
        .await;
    match outsider {
        Ok(None) => {}
        Ok(Some(_)) => panic!("another account could read the private issue"),
        Err(error) => panic!("outsider read failed: {error}"),
    }

    let conflict = service
        .submit(account_id, command(key, "Different problem"))
        .await;
    assert!(matches!(
        conflict,
        Err(SubmitIssueError::IdempotencyConflict)
    ));

    let mut connection = match pool.get().await {
        Ok(connection) => connection,
        Err(error) => panic!("test query connection failed: {error}"),
    };
    let counts = match sql_query(
        "SELECT
             (SELECT count(*) FROM civic.issues WHERE issue_id = $1) AS issues,
             (SELECT count(*) FROM private.issue_submission_context WHERE issue_id = $1) AS contexts,
             (SELECT count(*) FROM private.issue_submission_idempotency
               WHERE issue_id = $1
                 AND fingerprint_version = 1
                 AND octet_length(idempotency_key_hash) = 32
                 AND octet_length(request_fingerprint) = 32) AS idempotency,
             (SELECT count(*) FROM audit.events WHERE stream_id = $1 AND octet_length(event_hash) = 32) AS events,
             (SELECT count(*) FROM jobs.outbox_messages AS message
                JOIN audit.events AS event ON event.event_id = message.audit_event_id
               WHERE event.stream_id = $1
                 AND (SELECT count(*) FROM jsonb_object_keys(message.payload)) = 2
                 AND message.payload ? 'event_type'
                 AND message.payload ? 'issue_id') AS outbox,
             (SELECT count(*) FROM pg_trigger
               WHERE tgrelid = 'audit.events'::regclass
                 AND NOT tgisinternal
                 AND tgname IN ('audit_events_append_only', 'audit_events_reject_truncate')) AS audit_guards",
    )
    .bind::<diesel::sql_types::Uuid, _>(first.issue.id().into_uuid())
    .get_result::<AtomicCounts>(&mut connection)
    .await
    {
        Ok(counts) => counts,
        Err(error) => panic!("atomic row counts failed: {error}"),
    };
    assert_eq!(
        [
            counts.issues,
            counts.contexts,
            counts.idempotency,
            counts.events,
            counts.outbox,
            counts.audit_guards,
        ],
        [1, 1, 1, 1, 1, 2]
    );

    let mutation =
        sql_query("UPDATE audit.events SET event_type = event_type WHERE stream_id = $1")
            .bind::<diesel::sql_types::Uuid, _>(first.issue.id().into_uuid())
            .execute(&mut connection)
            .await;
    assert!(
        mutation.is_err(),
        "append-only audit mutation unexpectedly succeeded"
    );
}

fn command(key: IdempotencyKey, title: &str) -> SubmitIssueCommand {
    SubmitIssueCommand {
        idempotency_key: key,
        title: title.to_owned(),
        category_id: "road-surface".to_owned(),
        summary: "Buses avoid the damaged lane.".to_owned(),
        problem_statement: "The depression remains after repeated observations.".to_owned(),
        affected_community: "Passengers, cyclists, and drivers.".to_owned(),
        desired_outcome: "Restore a level and safe road surface.".to_owned(),
        longitude: -46.633_308,
        latitude: -23.550_52,
        geometry_source: GeometrySource::MapSelection,
        location_confirmed: true,
        location_label: Some("Eastbound bus lane".to_owned()),
        public_attribution: false,
        privacy_notice_version: "privacy-v1".to_owned(),
        privacy_notice_accepted: true,
    }
}

fn successful(
    result: Result<powerto_application::issues::SubmitIssueResult, SubmitIssueError>,
) -> powerto_application::issues::SubmitIssueResult {
    match result {
        Ok(result) => result,
        Err(error) => panic!("issue submission failed: {error}"),
    }
}

#[derive(QueryableByName)]
struct AtomicCounts {
    #[diesel(sql_type = BigInt)]
    issues: i64,
    #[diesel(sql_type = BigInt)]
    contexts: i64,
    #[diesel(sql_type = BigInt)]
    idempotency: i64,
    #[diesel(sql_type = BigInt)]
    events: i64,
    #[diesel(sql_type = BigInt)]
    outbox: i64,
    #[diesel(sql_type = BigInt)]
    audit_guards: i64,
}
