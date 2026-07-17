---
id: 0005-diesel-persistence
title: Diesel for Backend Persistence
---

# Diesel for Backend Persistence

## Status

ACCEPTED on 2026-07-16 by explicit project-owner decision.

## Context

The backend needs transactional persistence, spatial PostgreSQL queries, and
safe concurrent updates. The project owner chose Diesel instead of SQLx before
application code was created. Database migration ownership is decided
separately by ADR 0012.

## Decision

Use Diesel as the Rust persistence library and query builder. Diesel is an
infrastructure detail behind repository ports; its generated schema, query
types, and persistence records do not enter the domain or public API.

For the asynchronous Axum/Tokio proposal, use `diesel-async` with
`AsyncPgConnection` and a bounded pool after the first integration spike. If an
unsupported operation requires synchronous Diesel, run it in a dedicated
bounded blocking pool. Never perform blocking database I/O on a Tokio executor
thread.

Atlas owns versioned SQL migrations, their integrity file, revision history,
validation, and deployment. Diesel CLI is used only to derive and verify
`schema.rs` after Atlas has applied the migrations to a disposable database.
Diesel embedded migrations and `diesel migration` commands are not used.

Spatial operations use `postgis_diesel` when its typed surface is sufficient
and isolated, and parameterized `diesel::sql_query` statements when it is not.

## Consequences

### Positive

- Query and schema mismatches are caught early by Rust and Diesel.
- SQL migrations retain full PostgreSQL and PostGIS capability.
- The query DSL prevents string-concatenated SQL in ordinary repositories.
- Persistence can be tested and replaced without changing domain types.

### Negative

- Complex queries can produce difficult compiler errors and longer builds.
- Enabling Diesel's synchronous PostgreSQL connection would add a `libpq`
  runtime/build dependency; the initial `diesel-async` adapter deliberately
  enables only PostgreSQL backend types and uses `tokio-postgres` for I/O.
- PostGIS support relies on a community extension or locally defined SQL types
  and functions.
- Async execution adds `diesel-async` beside Diesel itself.
- The project must keep Atlas migrations and the derived Diesel schema in sync.

### Neutral

- Diesel is not a reason to model each table as a domain entity or expose CRUD
  endpoints.
- Raw SQL remains acceptable for migrations and isolated, parameterized spatial
  queries with integration tests.

## Compliance

- `domain` and `application` have no Diesel dependency.
- Diesel records and schema imports remain under the outer persistence adapter.
- CI applies migrations to a disposable PostgreSQL/PostGIS database, checks the
  generated schema, and runs repository integration tests.
- Atlas is the only process allowed to create or update migration revision
  state; the API and worker never migrate on startup.
- SQLx, SeaORM, and a second general-purpose persistence abstraction are not
  introduced without a superseding ADR.
- Every database call made from an async request or job is demonstrably
  non-blocking or isolated in the configured blocking pool.

## References

- [Diesel documentation](https://docs.rs/diesel/latest/diesel/)
- [Diesel getting started guide](https://diesel.rs/guides/getting-started.html)
- [`diesel-async` documentation](https://docs.rs/diesel-async/latest/diesel_async/)
- [`postgis_diesel` documentation](https://docs.rs/postgis_diesel/latest/postgis_diesel/)
- [ADR 0012: Atlas for Database Migrations](0012-atlas-database-migrations.md)
