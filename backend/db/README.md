# PowerTo database migrations

This directory is the authoritative PostgreSQL/PostGIS schema history. Atlas
owns migration execution; Diesel owns parameterized Rust persistence queries
and mappings. Neither the API nor the worker may run migrations at startup.

## Layout

```text
backend/db/
  atlas.hcl
  migrations/
    20260716000100_bootstrap.sql
    20260716000200_issue_intake.sql
    20260717000100_oidc_identity.sql
    atlas.sum
```

The bootstrap migration enables PostGIS and creates the `civic`, `private`,
`audit`, `jobs`, and `evidence` schemas. The issue-intake migration enables
`pgcrypto` and creates the first product tables. The identity migration adds
minimal private local accounts and OIDC issuer/subject mappings. No migration
creates roles, grants, or environment-specific objects.

## First vertical-slice schema

One issue submission is persisted atomically across these boundaries:

| Object | Purpose |
| --- | --- |
| `civic.issues` | Operational issue data, opaque UUIDv4 reference, lifecycle status, confirmed PostGIS point, and submission-policy version |
| `private.issue_submission_context` | Restricted submitter, geometry provenance, confirmed point, attribution consent, and privacy-notice version |
| `private.issue_submission_idempotency` | SHA-256 key digest plus a versioned normalized-command fingerprint; never the raw idempotency key |
| `audit.events` | Per-stream versioned canonical bytes with a PostgreSQL-generated SHA-256 hash chain and triggers rejecting update, delete, and truncate |
| `jobs.outbox_messages` | Privacy-minimal integration message linked to the committed audit event |
| `private.accounts` | Provider-independent local account and authorization status |
| `private.account_identities` | Minimal unique OIDC `(issuer, subject)` mapping; never tokens or profile claims |

The Diesel/`diesel-async` adapter writes all five records in one PostgreSQL
transaction. An idempotent replay reads the same issue's current owner-scoped
representation without writing again; reuse of the same key for a different
normalized command conflicts. Fingerprint version and digest are compared
before current submission policy is applied. Owner lookup joins the civic
record to its restricted context and filters both account and opaque reference
in the database query.

The concurrent reservation algorithm is tested under PostgreSQL's default
`READ COMMITTED` isolation: a competing `INSERT ... ON CONFLICT DO NOTHING`
waits for the winning transaction and then loads its committed idempotency row.
Changing transaction isolation requires a new concurrency test and review.

First-login identity provisioning uses a transaction-scoped advisory lock over
the issuer/subject pair, checks the mapping again after taking the lock, and
creates one UUIDv7 account plus one identity row. Concurrent valid logins
therefore converge without orphan accounts. Hash collisions only serialize
unrelated provisioning transactions; uniqueness constraints remain the source
of correctness.

Production must use a separate non-owner runtime role with only the required
schema/table privileges. In particular, it must not be able to update, delete,
truncate, disable triggers on, or change ownership of `audit.events`; the
migration owner is not an application credential. The triggers catch accidental
or compromised runtime mutations, but a database administrator can still
rewrite the database. Externally retained signed checkpoints remain required
before the audit chain can claim tamper evidence against that threat.

The outbox table is a durable transaction boundary, not a working delivery
pipeline yet: the Rust worker does not claim or process these rows. Likewise,
`category_key` is currently a syntax-validated slug, not a foreign key to a
canonical jurisdiction category, and this migration does not derive or store a
jurisdiction.

`civic.issues.public_location` currently holds the exact citizen-confirmed
civic problem point using the versioned method
`exact-civic-problem-point-v1`. It is not a device route, residence assertion,
or eligibility signal. No public issue-read endpoint exists in this slice; a
reviewed public projection and generalization policy are required before this
column can be used for public maps or lists.

## Prerequisites

- Atlas CLI v1.2.0, matching the version pinned by CI; review licensing before
  changing its distribution or enabling non-Community capabilities;
- PostgreSQL 18 with the PostGIS 3.6 extension package available;
- an empty database for a first apply;
- a dedicated migration role allowed to create the PostGIS extension and
  schemas; and
- `DATABASE_URL` supplied by the shell or deployment secret store.

Run commands from this directory:

```sh
cd backend/db
export DATABASE_URL='postgres://<migration-user>:<password>@<host>:5432/<database>?sslmode=require'
```

Never commit the real URL or print it in CI logs. Managed PostgreSQL services
must make PostGIS available to the migration role before the bootstrap runs.
If a provider requires an administrator to enable the extension first, prove
and document that bootstrap path before production; do not routinely bypass
Atlas with `--allow-dirty`.

## Validate

Verify the immutable directory checksum without connecting to a database:

```sh
atlas migrate validate --dir file://migrations
```

Validate SQL semantics by replaying the complete history against a disposable,
empty PostgreSQL/PostGIS database:

```sh
atlas migrate validate \
  --dir file://migrations \
  --dev-url "$ATLAS_DEV_DATABASE_URL"
```

`ATLAS_DEV_DATABASE_URL` must identify a disposable database matching the
production PostgreSQL and PostGIS major versions. Semantic validation can
create and inspect objects there; never point it at shared or production data.

When the selected Atlas edition supports it, also lint only the new files:

```sh
atlas migrate lint \
  --dir file://migrations \
  --dev-url "$ATLAS_DEV_DATABASE_URL" \
  --git-base origin/main
```

After adding or intentionally editing an unapplied migration, regenerate and
validate the checksum:

```sh
atlas migrate hash --dir file://migrations
atlas migrate validate --dir file://migrations
```

Commit the SQL file and `atlas.sum` together. Never edit, rename, reorder, or
delete an applied migration; add a forward correction instead.

## Inspect and apply

Review migration state and the exact pending SQL first:

```sh
atlas migrate status --env local
atlas migrate apply --env local --tx-mode file --dry-run
```

Apply from one serialized release job, never concurrently from application
replicas:

```sh
atlas migrate apply --env production --tx-mode file
atlas migrate status --env production
```

Atlas takes a PostgreSQL advisory lock by default. Keep that lock enabled and
ensure the deployment platform starts only one migration job. Production uses
linear, roll-forward history; recovery relies on a tested backup/restore and a
new corrective migration rather than assumed down migrations.

## Transaction policy

Atlas wraps each migration file in its own transaction by default. Every normal
file relies on that boundary and must not contain manual `BEGIN` or `COMMIT`.
The explicit `-- atlas:txmode file` header documents the policy in SQL as well.

A PostgreSQL operation that cannot run in a transaction, such as `CREATE INDEX
CONCURRENTLY`, belongs in a small, dedicated file headed with:

```sql
-- atlas:txmode none
```

Such a file requires a written retry and roll-forward plan plus dedicated
review. Large or destructive changes follow expand, backfill, verify, and
contract across compatible application releases.

## Atlas Community and Pro boundaries

The architecture depends only on local, versioned SQL migrations and the core
PostgreSQL workflow: hash/validate, status, diff where useful, and apply. It does
not require Atlas Cloud.

Atlas is open-core, and its distribution matters:

- the Apache-2.0 Community Edition supports core PostgreSQL schema management
  and versioned migrations, but the current upstream feature matrix excludes
  migration linting/testing, approval policies, hooks, drift detection,
  Registry features, and declarative modeling of extensions and several other
  advanced PostgreSQL objects;
- the standard CLI is distributed under the Atlas EULA and unlocks Pro
  capabilities through login/licensing; a command being present in the binary
  does not mean it is available under the selected plan; and
- this bootstrap uses hand-authored SQL for PostGIS, so correctness is proven by
  replay against a real disposable PostGIS database rather than depending on
  paid extension modeling.

Pin the exact Atlas version and distribution in development and CI. Treat Pro
linting and governance as additional gates, not as prerequisites for applying
the repository's migration history.

## References

- [Atlas versioned migrations](https://atlasgo.io/versioned/intro)
- [Migration directory integrity](https://atlasgo.io/concepts/migration-directory-integrity)
- [Applying migrations and transaction modes](https://atlasgo.io/versioned/apply)
- [Atlas Community Edition capability matrix](https://atlasgo.io/community-edition)
