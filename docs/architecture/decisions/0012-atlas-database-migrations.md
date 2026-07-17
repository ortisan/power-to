---
id: 0012-atlas-database-migrations
title: Atlas for Database Migrations
---

# Atlas for Database Migrations

## Status

ACCEPTED on 2026-07-16 by explicit project-owner decision.

## Context

Diesel is the accepted Rust persistence library, but it does not need to own
schema evolution. PowerTo requires reviewable PostgreSQL/PostGIS migrations,
linear history, integrity checks, CI simulation, and a deployment process that
does not race between API replicas.

## Decision

Use the Atlas CLI and its versioned migration workflow as the only database
migration mechanism. The repository layout is:

```text
backend/db/
  atlas.hcl
  migrations/
    <timestamp>_<name>.sql
    atlas.sum
```

The versioned SQL directory is the authoritative deployment history. Migration
files and `atlas.sum` are committed and reviewed together. Applied migrations
are immutable; corrections are new forward migrations.

Atlas may generate a migration with `atlas migrate diff` when it accurately
models the desired PostgreSQL object. PostGIS extensions, spatial objects,
complex constraints, data backfills, and operationally sensitive indexes may
use explicitly authored SQL. Generated SQL is always reviewed before commit.
The first database spike must prove which PostGIS and multi-schema operations
the selected Atlas edition supports; the design does not depend on Atlas Cloud or
a paid feature.

CI performs these gates against a disposable database with the production
PostgreSQL and PostGIS major versions:

1. validate the directory checksum and SQL with `atlas migrate validate`;
2. lint the branch migrations with `atlas migrate lint` where supported;
3. apply the entire history to an empty database;
4. run repository and migration integration tests;
5. derive `schema.rs` with `diesel print-schema` and fail on an uncommitted diff.

Production uses `atlas migrate apply` as a serialized release job with a
dedicated migration role. The API and worker neither embed migrations nor run
them at process startup. Secrets enter `atlas.hcl` through environment or the
deployment secret store and are never committed.

Atlas's default transaction-per-file mode is retained. A statement that cannot
run in a transaction, such as a concurrent PostgreSQL index build, requires an
explicit file-level transaction directive, a retry plan, and dedicated review.
Large and destructive changes follow expand, backfill, verify, contract across
compatible releases. Production recovery favors roll-forward plus a verified
backup/restore plan over assumed down migrations.

## Consequences

### Positive

- Migration generation, integrity, revision tracking, validation, and apply
  have one owner.
- SQL remains visible and can use PostgreSQL/PostGIS features directly.
- Atlas simulates the full history before release.
- Database rollout is independent of API replica startup.

### Negative

- Developers need both Atlas and Diesel CLI in the toolchain.
- Some advanced PostgreSQL objects may need hand-authored SQL or paid Atlas
  capabilities; they must be proven rather than assumed.
- `schema.rs` drift becomes a CI concern that requires deterministic generation.

### Neutral

- Diesel still owns typed Rust queries and mappings, not migration execution.
- A declarative desired-state file may be added later, but it does not replace
  reviewed versioned migration files in deployments.

## Compliance

- `diesel migration`, Diesel embedded migrations, and application startup
  migrations are prohibited.
- CI rejects a changed historical migration checksum.
- The release job records Atlas status before and after apply and is single-run.
- Every contract migration documents its compatible application versions and
  backup or roll-forward procedure.
- Development and CI must use the same exact Atlas version and distribution.
  This foundation does not yet contain that tooling pin, so adding it is a
  release-readiness requirement rather than a completed control.

## References

- [Atlas versioned migration diff](https://atlasgo.io/versioned/diff)
- [Atlas migration apply workflow](https://atlasgo.io/versioned/apply)
- [Atlas migration safety linting](https://atlasgo.io/versioned/lint)
- [Atlas migration directory integrity](https://atlasgo.io/concepts/migration-directory-integrity)
- [Atlas open-source repository](https://github.com/ariga/atlas)
