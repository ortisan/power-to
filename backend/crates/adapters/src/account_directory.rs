use async_trait::async_trait;
use diesel::{OptionalExtension as _, QueryableByName, result::Error as DieselError, sql_query};
use diesel_async::{AsyncConnection as _, AsyncPgConnection, RunQueryDsl as _};
use powerto_application::identity::{AccountDirectory, AccountDirectoryError, ExternalIdentity};
use powerto_domain::AccountId;
use uuid::Uuid;

use crate::database::PgPool;

/// Diesel/PostgreSQL directory for verified external identities.
#[derive(Clone, Debug)]
pub struct PostgresAccountDirectory {
    pool: PgPool,
}

impl PostgresAccountDirectory {
    /// Uses the existing application pool. Atlas remains the migration owner.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn connection(
        &self,
    ) -> Result<
        diesel_async::pooled_connection::bb8::PooledConnection<'_, AsyncPgConnection>,
        AccountDirectoryError,
    > {
        self.pool.get().await.map_err(|_| {
            tracing::warn!(
                operation = "identity_directory.acquire",
                "account directory unavailable"
            );
            AccountDirectoryError::Unavailable
        })
    }
}

#[async_trait]
impl AccountDirectory for PostgresAccountDirectory {
    async fn resolve_or_provision(
        &self,
        identity: &ExternalIdentity,
    ) -> Result<AccountId, AccountDirectoryError> {
        let mut connection = self.connection().await?;
        let candidate = AccountId::new();
        let row = connection
            .transaction(async |connection| {
                resolve_or_provision_transaction(connection, identity, candidate).await
            })
            .await
            .map_err(database_error)?;

        match row.status.as_str() {
            "active" => Ok(AccountId::from_uuid(row.account_id)),
            "suspended" | "closed" => Err(AccountDirectoryError::Forbidden),
            _ => {
                tracing::error!(
                    operation = "identity_directory.resolve",
                    "account directory returned an invalid status"
                );
                Err(AccountDirectoryError::InvalidStoredData)
            }
        }
    }
}

async fn resolve_or_provision_transaction(
    connection: &mut AsyncPgConnection,
    identity: &ExternalIdentity,
    candidate: AccountId,
) -> Result<AccountRow, DieselError> {
    if let Some(row) = load_account(connection, identity).await? {
        return Ok(row);
    }

    // A transaction-scoped advisory lock makes first-login provisioning
    // deterministic. Hash collisions only serialize unrelated identities.
    sql_query(
        "SELECT pg_advisory_xact_lock(
             hashtextextended(
                 jsonb_build_array($1::text, $2::text)::text,
                 5787213827046133845
             )
         )::text AS locked",
    )
    .bind::<diesel::sql_types::Text, _>(identity.issuer())
    .bind::<diesel::sql_types::Text, _>(identity.subject())
    .get_result::<AdvisoryLockRow>(connection)
    .await?;

    if let Some(row) = load_account(connection, identity).await? {
        return Ok(row);
    }

    sql_query(
        "INSERT INTO private.accounts (account_id)
         VALUES ($1)",
    )
    .bind::<diesel::sql_types::Uuid, _>(candidate.into_uuid())
    .execute(connection)
    .await?;
    sql_query(
        "INSERT INTO private.account_identities (issuer, subject, account_id)
         VALUES ($1, $2, $3)",
    )
    .bind::<diesel::sql_types::Text, _>(identity.issuer())
    .bind::<diesel::sql_types::Text, _>(identity.subject())
    .bind::<diesel::sql_types::Uuid, _>(candidate.into_uuid())
    .execute(connection)
    .await?;

    Ok(AccountRow {
        account_id: candidate.into_uuid(),
        status: "active".to_owned(),
    })
}

async fn load_account(
    connection: &mut AsyncPgConnection,
    identity: &ExternalIdentity,
) -> Result<Option<AccountRow>, DieselError> {
    sql_query(
        "SELECT account.account_id, account.status
         FROM private.account_identities AS identity
         JOIN private.accounts AS account USING (account_id)
         WHERE identity.issuer = $1 AND identity.subject = $2",
    )
    .bind::<diesel::sql_types::Text, _>(identity.issuer())
    .bind::<diesel::sql_types::Text, _>(identity.subject())
    .get_result::<AccountRow>(connection)
    .await
    .optional()
}

#[derive(QueryableByName)]
struct AccountRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    account_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    status: String,
}

#[derive(QueryableByName)]
struct AdvisoryLockRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    #[allow(dead_code)]
    locked: String,
}

fn database_error(_error: DieselError) -> AccountDirectoryError {
    tracing::warn!(
        operation = "identity_directory.resolve",
        "account directory query failed"
    );
    AccountDirectoryError::Unavailable
}
