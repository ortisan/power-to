use std::{env, num::NonZeroU32, time::Duration};

use diesel::{QueryableByName, sql_query, sql_types::BigInt};
use diesel_async::RunQueryDsl as _;
use powerto_adapters::{account_directory::PostgresAccountDirectory, database::create_pool};
use powerto_application::identity::{
    AccountDirectory as _, AccountDirectoryError, ExternalIdentity,
};

#[tokio::test]
#[ignore = "requires POWERTO_TEST_DATABASE_URL pointing at a disposable migrated database"]
async fn identity_provisioning_is_atomic_and_respects_account_status() {
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
    let directory = PostgresAccountDirectory::new(pool.clone());
    let identity = match ExternalIdentity::new(
        "https://identity.test/realms/civic".to_owned(),
        "integration-subject".to_owned(),
    ) {
        Ok(identity) => identity,
        Err(error) => panic!("test identity should be valid: {error}"),
    };

    let first = directory.resolve_or_provision(&identity);
    let second = directory.resolve_or_provision(&identity);
    let (first, second) = tokio::join!(first, second);
    let first = match first {
        Ok(account_id) => account_id,
        Err(error) => panic!("first identity resolution failed: {error}"),
    };
    let second = match second {
        Ok(account_id) => account_id,
        Err(error) => panic!("second identity resolution failed: {error}"),
    };
    assert!(first == second);
    assert_eq!(first.into_uuid().get_version_num(), 7);

    let mut connection = match pool.get().await {
        Ok(connection) => connection,
        Err(error) => panic!("test query connection failed: {error}"),
    };
    let counts = match sql_query(
        "SELECT
             (SELECT count(*) FROM private.accounts WHERE account_id = $1) AS accounts,
             (SELECT count(*) FROM private.account_identities WHERE account_id = $1) AS identities",
    )
    .bind::<diesel::sql_types::Uuid, _>(first.into_uuid())
    .get_result::<IdentityCounts>(&mut connection)
    .await
    {
        Ok(counts) => counts,
        Err(error) => panic!("identity row counts failed: {error}"),
    };
    assert_eq!([counts.accounts, counts.identities], [1, 1]);

    if let Err(error) = sql_query(
        "UPDATE private.accounts
         SET status = 'suspended', updated_at = transaction_timestamp(), version = version + 1
         WHERE account_id = $1",
    )
    .bind::<diesel::sql_types::Uuid, _>(first.into_uuid())
    .execute(&mut connection)
    .await
    {
        panic!("test account suspension failed: {error}");
    }
    assert!(matches!(
        directory.resolve_or_provision(&identity).await,
        Err(AccountDirectoryError::Forbidden)
    ));
}

#[derive(QueryableByName)]
struct IdentityCounts {
    #[diesel(sql_type = BigInt)]
    accounts: i64,
    #[diesel(sql_type = BigInt)]
    identities: i64,
}
